import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useToastStore } from "./toast";

export interface RagReference {
  file_id: number;
  path: string;
  name: string;
  snippet: string | null;
  score: number;
  source: string;
}

export interface RagContext {
  references: RagReference[];
}

export interface ToolCallEntry {
  callId: string;
  name: string;
  arguments: string;
  status: "pending-confirm" | "running" | "ok" | "error" | "denied" | "undone";
  result?: string;
  error?: string;
  durationMs?: number;
  operationId?: number;
  undoable?: boolean;
}

export interface PendingConfirmation {
  callId: string;
  conversationId: number;
  messageId: number;
  name: string;
  arguments: string;
}

export interface ChatMessage {
  id?: number;
  role: "user" | "assistant" | "system";
  content: string;
  rag?: RagContext | null;
  streaming?: boolean;
  toolCalls?: ToolCallEntry[];
}

export interface ConversationSummary {
  id: number;
  title: string;
  created_at: number;
  updated_at: number;
}

export const useChatStore = defineStore("chat", () => {
  const messages = ref<ChatMessage[]>([]);
  const conversationId = ref<number | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const conversations = ref<ConversationSummary[]>([]);
  const useRag = ref(true);
  const panelOpen = ref(false);
  const pendingConfirmations = ref<PendingConfirmation[]>([]);

  const unlisteners: UnlistenFn[] = [];
  let listenersReady = false;
  let listenersPromise: Promise<void> | null = null;

  const hasMessages = computed(() => messages.value.length > 0);

  function findStreamingMessage(): ChatMessage | null {
    for (let i = messages.value.length - 1; i >= 0; i--) {
      const m = messages.value[i];
      if (m.role === "assistant" && m.streaming) return m;
    }
    return null;
  }

  function pushToolCall(msg: ChatMessage, callId: string, name: string, args: string) {
    if (!msg.toolCalls) msg.toolCalls = [];
    if (!msg.toolCalls.find((t) => t.callId === callId)) {
      msg.toolCalls.push({ callId, name, arguments: args, status: "running" });
    }
  }

  function markToolPendingConfirm(msg: ChatMessage, callId: string) {
    if (!msg.toolCalls) return;
    const existing = msg.toolCalls.find((t) => t.callId === callId);
    if (existing) existing.status = "pending-confirm";
  }

  function markToolDenied(msg: ChatMessage, callId: string) {
    if (!msg.toolCalls) return;
    const existing = msg.toolCalls.find((t) => t.callId === callId);
    if (existing) existing.status = "denied";
  }

  function completeToolCall(
    msg: ChatMessage,
    callId: string,
    result: string,
    error: string | null,
    durationMs: number,
  ) {
    if (!msg.toolCalls) msg.toolCalls = [];
    let operationId: number | undefined;
    let undoable: boolean | undefined;
    if (!error && result) {
      try {
        const parsed = JSON.parse(result);
        if (typeof parsed.operation_id === "number") operationId = parsed.operation_id;
        if (typeof parsed.undoable === "boolean") undoable = parsed.undoable;
      } catch {
        /* ignore */
      }
    }
    const existing = msg.toolCalls.find((t) => t.callId === callId);
    if (existing) {
      existing.status = error ? "error" : "ok";
      existing.result = result;
      existing.error = error || undefined;
      existing.durationMs = durationMs;
      existing.operationId = operationId;
      existing.undoable = undoable;
    } else {
      msg.toolCalls.push({
        callId,
        name: "",
        arguments: "",
        status: error ? "error" : "ok",
        result,
        error: error || undefined,
        durationMs,
        operationId,
        undoable,
      });
    }
  }

  async function ensureListeners() {
    if (listenersReady) return;
    if (listenersPromise) return listenersPromise;
    listenersPromise = (async () => {
      const deltaUnlisten = await listen<{
        conversation_id: number;
        message_id: number;
        delta: string;
      }>("chat-delta", (e) => {
        const { delta } = e.payload;
        const target = findStreamingMessage();
        if (target) target.content += delta;
      });
      unlisteners.push(deltaUnlisten);

      const doneUnlisten = await listen<{
        conversation_id: number;
        message_id: number;
        input_tokens: number | null;
        output_tokens: number | null;
        error: string | null;
      }>("chat-done", (e) => {
        const { error: err } = e.payload;
        loading.value = false;
        const target = findStreamingMessage();
        if (target) {
          target.streaming = false;
          if (err) error.value = err;
        }
      });
      unlisteners.push(doneUnlisten);

      const toolCallUnlisten = await listen<{
        conversation_id: number;
        message_id: number;
        call_id: string;
        name: string;
        arguments: string;
      }>("agent-tool-call", (e) => {
        const { call_id, name, arguments: args } = e.payload;
        const target = findStreamingMessage();
        if (target) pushToolCall(target, call_id, name, args);
      });
      unlisteners.push(toolCallUnlisten);

      const toolResultUnlisten = await listen<{
        conversation_id: number;
        message_id: number;
        call_id: string;
        result: string;
        error: string | null;
        duration_ms: number;
      }>("agent-tool-result", (e) => {
        const { call_id, result, error, duration_ms } = e.payload;
        const target = findStreamingMessage();
        if (target) completeToolCall(target, call_id, result, error, duration_ms);
      });
      unlisteners.push(toolResultUnlisten);

      const toolConfirmUnlisten = await listen<{
        conversation_id: number;
        message_id: number;
        call_id: string;
        name: string;
        arguments: string;
      }>("agent-tool-confirm", (e) => {
        const { conversation_id, message_id, call_id, name, arguments: args } = e.payload;
        const target = findStreamingMessage();
        if (target) markToolPendingConfirm(target, call_id);
        pendingConfirmations.value.push({
          callId: call_id,
          conversationId: conversation_id,
          messageId: message_id,
          name,
          arguments: args,
        });
      });
      unlisteners.push(toolConfirmUnlisten);

      listenersReady = true;
    })();
    return listenersPromise;
  }

  async function respondConfirmation(callId: string, approved: boolean) {
    const pending = pendingConfirmations.value.find((p) => p.callId === callId);
    if (!pending) return;
    try {
      const found = await invoke<boolean>("tool_confirm", {
        conversationId: pending.conversationId,
        callId,
        approved,
      });
      if (!found) {
        console.warn("tool_confirm: backend has no pending entry for", callId);
      }
      pendingConfirmations.value = pendingConfirmations.value.filter((p) => p.callId !== callId);
      if (!approved) {
        const target = findStreamingMessage();
        if (target) markToolDenied(target, callId);
      }
    } catch (e) {
      console.warn("tool_confirm failed:", e);
      const toast = useToastStore();
      toast.error("确认失败：" + String(e));
    }
  }

  async function undoOperation(operationId: number, callIdHint?: string): Promise<boolean> {
    try {
      await invoke("undo_operation", { operationId });
      for (const m of messages.value) {
        if (!m.toolCalls) continue;
        for (const t of m.toolCalls) {
          if (t.operationId === operationId || (callIdHint && t.callId === callIdHint)) {
            t.status = "undone";
          }
        }
      }
      return true;
    } catch (e) {
      error.value = String(e);
      return false;
    }
  }

  async function send(text: string) {
    const trimmed = text.trim();
    if (!trimmed || loading.value) return;

    await ensureListeners();
    error.value = null;

    messages.value.push({ role: "user", content: trimmed });
    const assistantMsg: ChatMessage = { role: "assistant", content: "", streaming: true };
    messages.value.push(assistantMsg);

    loading.value = true;

    invoke<{
      conversation_id: number;
      message_id: number;
      rag_context: RagContext | null;
    }>("chat_send", {
      conversationId: conversationId.value,
      message: trimmed,
      useRag: useRag.value,
    })
      .then((result) => {
        conversationId.value = result.conversation_id;
        assistantMsg.id = result.message_id;
        assistantMsg.rag = result.rag_context;
        loadConversations();
      })
      .catch((e) => {
        error.value = String(e);
        loading.value = false;
        assistantMsg.streaming = false;
        if (!assistantMsg.content) {
          assistantMsg.content = `[错误] ${String(e)}`;
        }
      });
  }

  async function stopGenerate() {
    if (!loading.value) return;
    const convId = conversationId.value;
    try {
      await invoke("chat_stop", { conversationId: convId });
    } catch (e) {
      console.warn("stop failed:", e);
    }
    loading.value = false;
    for (const m of messages.value) {
      if (m.streaming) {
        m.streaming = false;
        if (!m.content) m.content = "[已中断]";
      }
    }
    if (pendingConfirmations.value.length) {
      for (const p of pendingConfirmations.value) {
        const target = findStreamingMessage();
        if (target) markToolDenied(target, p.callId);
      }
      pendingConfirmations.value = [];
    }
  }

  async function loadConversation(id: number) {
    conversationId.value = id;
    messages.value = [];
    try {
      const history = await invoke<Array<{ role: string; content: string }>>("list_messages", {
        conversationId: id,
      });
      messages.value = history.map((m) => ({
        role: m.role as ChatMessage["role"],
        content: m.content,
      }));
    } catch (e) {
      console.warn("load history failed:", e);
    }
  }

  async function loadConversations() {
    try {
      const list = await invoke<ConversationSummary[]>("list_conversations");
      conversations.value = list;
    } catch {
      /* ignore */
    }
  }

  async function deleteConversation(id: number) {
    try {
      await invoke("delete_conversation", { conversationId: id });
      if (conversationId.value === id) {
        newConversation();
      }
      await loadConversations();
    } catch (e) {
      error.value = String(e);
    }
  }

  async function renameConversation(id: number, title: string) {
    try {
      await invoke("rename_conversation", { conversationId: id, title });
      await loadConversations();
    } catch (e) {
      error.value = String(e);
    }
  }

  function newConversation() {
    conversationId.value = null;
    messages.value = [];
    error.value = null;
    pendingConfirmations.value = [];
  }

  function togglePanel() {
    panelOpen.value = !panelOpen.value;
    if (panelOpen.value) loadConversations();
  }

  return {
    messages,
    conversationId,
    loading,
    error,
    conversations,
    useRag,
    panelOpen,
    hasMessages,
    pendingConfirmations,
    send,
    stopGenerate,
    loadConversation,
    loadConversations,
    deleteConversation,
    renameConversation,
    newConversation,
    togglePanel,
    ensureListeners,
    respondConfirmation,
    undoOperation,
  };
});
