import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

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

export interface ChatMessage {
  id?: number;
  role: "user" | "assistant" | "system";
  content: string;
  rag?: RagContext | null;
  streaming?: boolean;
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

  let unlistenDelta: UnlistenFn | null = null;
  let unlistenDone: UnlistenFn | null = null;

  const hasMessages = computed(() => messages.value.length > 0);

  function findStreamingMessage(): ChatMessage | null {
    for (let i = messages.value.length - 1; i >= 0; i--) {
      const m = messages.value[i];
      if (m.role === "assistant" && m.streaming) return m;
    }
    return null;
  }

  async function ensureListeners() {
    if (unlistenDelta && unlistenDone) return;
    unlistenDelta = await listen<{ conversation_id: number; message_id: number; delta: string }>(
      "chat-delta",
      (e) => {
        const { delta } = e.payload;
        const target = findStreamingMessage();
        if (target) target.content += delta;
      },
    );
    unlistenDone = await listen<{
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
        assistantMsg.content = `[错误] ${e}`;
      });
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
    send,
    loadConversation,
    loadConversations,
    deleteConversation,
    renameConversation,
    newConversation,
    togglePanel,
    ensureListeners,
  };
});
