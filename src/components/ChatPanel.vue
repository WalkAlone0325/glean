<script setup lang="ts">
import { computed, nextTick, onMounted, ref, useTemplateRef, watch } from "vue";
import { useChatStore, type ChatMessage } from "../stores/chat";
import { invoke } from "@tauri-apps/api/core";
import {
  X,
  Send,
  Loader2,
  Sparkles,
  MessageSquarePlus,
  ExternalLink,
  FolderOpen,
  History,
  Trash2,
  Pencil,
  Check,
  Square,
  CheckCircle2,
  AlertCircle,
  Undo2,
} from "@lucide/vue";
import { renderMarkdown } from "../utils/markdown";
import { useToastStore } from "../stores/toast";
import "highlight.js/styles/github-dark.css";

const chat = useChatStore();
const toast = useToastStore();
const input = ref("");
const scrollRef = useTemplateRef<HTMLDivElement>("scrollRef");
const inputRef = useTemplateRef<HTMLTextAreaElement>("inputRef");
const showHistory = ref(false);
const editingId = ref<number | null>(null);
const editingTitle = ref("");
const showScrollToBottom = ref(false);

function autoResize() {
  const el = inputRef.value;
  if (!el) return;
  el.style.height = "auto";
  el.style.height = Math.min(el.scrollHeight, 160) + "px";
}

watch(input, async () => {
  await nextTick();
  autoResize();
});

onMounted(() => {
  chat.ensureListeners();
  chat.loadConversations();
  nextTick(() => inputRef.value?.focus());
});

watch(
  () => chat.panelOpen,
  (v) => {
    if (v) nextTick(() => inputRef.value?.focus());
  },
);

watch(
  () => chat.messages.length,
  async () => {
    await nextTick();
    if (scrollRef.value) scrollRef.value.scrollTop = scrollRef.value.scrollHeight;
  },
);

watch(
  () => chat.messages[chat.messages.length - 1]?.content,
  async () => {
    await nextTick();
    if (scrollRef.value && !showScrollToBottom.value) {
      scrollRef.value.scrollTop = scrollRef.value.scrollHeight;
    }
  },
);

function onScroll() {
  const el = scrollRef.value;
  if (!el) return;
  const distanceToBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
  showScrollToBottom.value = distanceToBottom > 120;
}

function scrollToBottom() {
  if (scrollRef.value) scrollRef.value.scrollTop = scrollRef.value.scrollHeight;
}

function onSend() {
  const text = input.value;
  if (!text.trim()) return;
  input.value = "";
  nextTick(() => autoResize());
  chat.send(text);
}

function onStop() {
  chat.stopGenerate();
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    onSend();
  }
}

function userBubble(msg: ChatMessage) {
  return msg.role === "user";
}

function openFile(path: string) {
  invoke("open_file", { path });
}

function revealInFinder(path: string) {
  invoke("reveal_in_finder", { path });
}

function assistantHtml(content: string): string {
  return renderMarkdown(content);
}

function formatArgs(raw: string): string {
  if (!raw) return "";
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw;
  }
}

function formatResult(raw: string): string {
  if (!raw) return "";
  try {
    const parsed = JSON.parse(raw);
    return JSON.stringify(parsed, null, 2);
  } catch {
    return raw;
  }
}

function onRespond(approved: boolean) {
  const first = chat.pendingConfirmations?.[0];
  if (!first) return;
  chat.respondConfirmation(first.callId, approved);
}

async function onUndo(operationId: number) {
  const ok = await chat.undoOperation(operationId);
  if (ok) toast.success("已撤销");
  else toast.error("撤销失败：" + (chat.error || "未知错误"));
}

function formatTime(ts: number): string {
  if (!ts) return "";
  const d = new Date(ts * 1000);
  const now = new Date();
  const diff = (now.getTime() - d.getTime()) / 1000;
  if (diff < 60) return "刚刚";
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`;
  if (diff < 7 * 86400) return `${Math.floor(diff / 86400)} 天前`;
  return d.toLocaleDateString("zh-CN", { month: "2-digit", day: "2-digit" });
}

async function selectConversation(id: number) {
  await chat.loadConversation(id);
  showHistory.value = false;
}

function startEdit(id: number, currentTitle: string) {
  editingId.value = id;
  editingTitle.value = currentTitle;
}

async function saveEdit() {
  if (editingId.value !== null && editingTitle.value.trim()) {
    await chat.renameConversation(editingId.value, editingTitle.value.trim());
  }
  editingId.value = null;
}

async function onDelete(id: number) {
  if (confirm("确定删除这个对话吗？")) {
    await chat.deleteConversation(id);
  }
}

const canSend = computed(() => !chat.loading && input.value.trim().length > 0);
</script>

<template>
  <aside
    v-if="chat.panelOpen"
    class="flex h-full w-96 flex-col border-l border-border bg-background"
  >
    <div class="flex items-center justify-between border-b border-border px-4 py-2.5">
      <div class="flex items-center gap-2">
        <Sparkles class="size-4 text-primary" />
        <h2 class="text-sm font-semibold">AI 助手</h2>
        <label class="ml-2 flex cursor-pointer items-center gap-1 text-[11px] text-muted-foreground">
          <input v-model="chat.useRag" type="checkbox" class="size-3" />
          RAG
        </label>
      </div>
      <div class="flex items-center gap-1">
        <button
          :class="[
            'rounded-md p-1 hover:bg-muted',
            showHistory ? 'text-primary bg-muted' : 'text-muted-foreground',
          ]"
          title="历史对话"
          @click="showHistory = !showHistory"
        >
          <History class="size-4" />
        </button>
        <button
          class="rounded-md p-1 text-muted-foreground hover:bg-muted"
          title="新对话"
          @click="chat.newConversation()"
        >
          <MessageSquarePlus class="size-4" />
        </button>
        <button
          class="rounded-md p-1 text-muted-foreground hover:bg-muted"
          title="关闭"
          @click="chat.togglePanel()"
        >
          <X class="size-4" />
        </button>
      </div>
    </div>

    <div class="flex flex-1 overflow-hidden">
      <div
        v-if="showHistory"
        class="w-44 shrink-0 overflow-auto border-r border-border bg-muted/30 py-2"
      >
        <div class="px-3 pb-1 text-[10px] uppercase tracking-wide text-muted-foreground">
          历史对话 ({{ chat.conversations.length }})
        </div>
        <div
          v-for="conv in chat.conversations"
          :key="conv.id"
          :class="[
            'group flex cursor-pointer items-start gap-1 px-2 py-1.5 hover:bg-muted',
            conv.id === chat.conversationId ? 'bg-muted' : '',
          ]"
          @click="selectConversation(conv.id)"
        >
          <div class="min-w-0 flex-1">
            <div v-if="editingId === conv.id" class="flex gap-1" @click.stop>
              <input
                v-model="editingTitle"
                class="w-full rounded bg-background px-1 py-0.5 text-xs outline-none ring-1 ring-primary"
                autofocus
                @keydown.enter="saveEdit"
                @keydown.escape="editingId = null"
              />
              <button class="text-primary" @click.stop="saveEdit">
                <Check class="size-3" />
              </button>
            </div>
            <template v-else>
              <div class="truncate text-xs font-medium">{{ conv.title || "新对话" }}</div>
              <div class="text-[10px] text-muted-foreground">{{ formatTime(conv.updated_at) }}</div>
            </template>
          </div>
          <div v-if="editingId !== conv.id" class="hidden gap-0.5 group-hover:flex">
            <button
              class="rounded p-0.5 text-muted-foreground hover:bg-background hover:text-foreground"
              title="重命名"
              @click.stop="startEdit(conv.id, conv.title)"
            >
              <Pencil class="size-3" />
            </button>
            <button
              class="rounded p-0.5 text-muted-foreground hover:bg-background hover:text-red-500"
              title="删除"
              @click.stop="onDelete(conv.id)"
            >
              <Trash2 class="size-3" />
            </button>
          </div>
        </div>
        <div
          v-if="!chat.conversations.length"
          class="px-3 py-4 text-center text-[11px] text-muted-foreground"
        >
          暂无历史对话
        </div>
      </div>

      <div ref="scrollRef" class="relative flex-1 overflow-auto p-4" @scroll="onScroll">
        <div
          v-if="!chat.hasMessages"
          class="flex h-full items-center justify-center text-xs text-muted-foreground"
        >
          输入问题开始对话
        </div>
        <div v-else class="space-y-3">
          <div
            v-for="(msg, idx) in chat.messages"
            :key="idx"
            :class="['flex', userBubble(msg) ? 'justify-end' : 'justify-start']"
          >
            <div
              :class="[
                'max-w-[85%] break-words rounded-lg px-3 py-2 text-sm',
                userBubble(msg)
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-muted text-foreground',
              ]"
            >
              <div v-if="userBubble(msg)" class="whitespace-pre-wrap">{{ msg.content }}</div>
              <template v-else>
                <div
                  v-if="msg.toolCalls && msg.toolCalls.length"
                  class="mb-2 space-y-1.5"
                >
                  <details
                    v-for="(tc, tidx) in msg.toolCalls"
                    :key="tidx"
                    class="rounded border border-border/60 bg-background/40 text-[11px]"
                  >
                    <summary class="flex cursor-pointer items-center gap-1.5 px-2 py-1 hover:bg-muted/40">
                      <Loader2
                        v-if="tc.status === 'running'"
                        class="size-3 animate-spin text-muted-foreground"
                      />
                      <Loader2
                        v-else-if="tc.status === 'pending-confirm'"
                        class="size-3 animate-spin text-yellow-500"
                      />
                      <CheckCircle2
                        v-else-if="tc.status === 'ok'"
                        class="size-3 text-emerald-500"
                      />
                      <AlertCircle
                        v-else-if="tc.status === 'denied'"
                        class="size-3 text-yellow-600"
                      />
                      <AlertCircle v-else class="size-3 text-red-500" />
                      <span class="font-medium">{{ tc.name || "(unknown)" }}</span>
                      <span
                        v-if="tc.status === 'pending-confirm'"
                        class="ml-auto text-[10px] text-yellow-600"
                      >等待确认</span>
                      <span
                        v-else-if="tc.status === 'denied'"
                        class="ml-auto text-[10px] text-yellow-600"
                      >已拒绝</span>
                      <span
                        v-else-if="tc.status === 'undone'"
                        class="ml-auto text-[10px] text-emerald-600"
                      >已撤销</span>
                      <span v-else-if="tc.durationMs !== undefined" class="ml-auto text-[10px] opacity-60">
                        {{ tc.durationMs }}ms
                      </span>
                      <button
                        v-if="tc.status === 'ok' && tc.undoable && tc.operationId !== undefined"
                        class="ml-2 flex items-center gap-0.5 rounded border border-border bg-background px-1.5 py-0.5 text-[10px] hover:bg-muted"
                        title="撤销此操作"
                        @click.stop="onUndo(tc.operationId)"
                      >
                        <Undo2 class="size-3" />
                        撤销
                      </button>
                    </summary>
                    <div class="space-y-1 border-t border-border/40 px-2 py-1.5">
                      <div v-if="tc.arguments" class="text-[10px] opacity-70">
                        <div class="mb-0.5 font-medium">参数</div>
                        <pre class="max-h-32 overflow-auto whitespace-pre-wrap break-all rounded bg-muted/60 p-1.5 font-mono text-[10px]">{{ formatArgs(tc.arguments) }}</pre>
                      </div>
                      <div v-if="tc.error" class="text-[10px] text-red-500">
                        <div class="mb-0.5 font-medium">错误</div>
                        <pre class="whitespace-pre-wrap break-all">{{ tc.error }}</pre>
                      </div>
                      <div v-else-if="tc.result" class="text-[10px] opacity-70">
                        <div class="mb-0.5 font-medium">结果</div>
                        <pre class="max-h-48 overflow-auto whitespace-pre-wrap break-all rounded bg-muted/60 p-1.5 font-mono text-[10px]">{{ formatResult(tc.result) }}</pre>
                      </div>
                    </div>
                  </details>
                </div>
                <div class="markdown-body" v-html="assistantHtml(msg.content)" />
              </template>
              <Loader2
                v-if="msg.streaming"
                class="ml-1 inline-block size-3 animate-spin align-text-bottom"
              />
              <div
                v-if="msg.rag && msg.rag.references.length"
                class="mt-2 border-t border-border/40 pt-2"
              >
                <div class="mb-1 text-[10px] opacity-70">引用文件</div>
                <div class="space-y-1">
                  <div
                    v-for="(ref, ridx) in msg.rag.references"
                    :key="ridx"
                    class="flex items-center gap-1 rounded bg-background/50 px-2 py-1 text-[11px]"
                  >
                    <span class="flex-1 truncate" :title="ref.path">{{ ref.name }}</span>
                    <span class="text-[10px] opacity-50">[{{ ridx + 1 }}]</span>
                    <button
                      class="rounded p-0.5 hover:bg-muted/80"
                      title="打开"
                      @click="openFile(ref.path)"
                    >
                      <ExternalLink class="size-3" />
                    </button>
                    <button
                      class="rounded p-0.5 hover:bg-muted/80"
                      title="在 Finder 中显示"
                      @click="revealInFinder(ref.path)"
                    >
                      <FolderOpen class="size-3" />
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
          <div v-if="chat.error" class="text-xs text-red-500">{{ chat.error }}</div>
        </div>
        <button
          v-if="showScrollToBottom"
          class="absolute bottom-4 left-1/2 -translate-x-1/2 rounded-full bg-background px-3 py-1 text-[11px] shadow-md border border-border hover:bg-muted"
          @click="scrollToBottom"
        >
          ↓ 回到底部
        </button>
      </div>
    </div>

    <div class="border-t border-border p-3">
      <div class="flex gap-2">
        <textarea
          ref="inputRef"
          v-model="input"
          placeholder="问任何问题... (Enter 发送, Shift+Enter 换行)"
          rows="1"
          class="flex-1 resize-none rounded-md border border-border bg-background px-3 py-2 text-sm outline-none focus:border-primary"
          style="min-height: 38px; max-height: 160px"
          @keydown="onKeydown"
        />
        <button
          v-if="chat.loading"
          class="flex items-center justify-center rounded-md bg-red-500/80 px-3 text-white hover:bg-red-500"
          title="停止生成"
          @click="onStop"
        >
          <Square class="size-3.5 fill-current" />
        </button>
        <button
          v-else
          :disabled="!canSend"
          class="flex items-center justify-center rounded-md bg-primary px-3 text-primary-foreground hover:opacity-90 disabled:opacity-50"
          @click="onSend"
        >
          <Send class="size-4" />
        </button>
      </div>
      <div class="mt-1.5 text-[10px] text-muted-foreground">
        <span v-if="chat.conversationId">对话 #{{ chat.conversationId }}</span>
        <span v-else>新对话</span>
      </div>
    </div>

    <Teleport to="body">
      <div
        v-if="chat.pendingConfirmations && chat.pendingConfirmations.length"
        class="fixed inset-0 z-[200] flex items-center justify-center bg-black/40 backdrop-blur-sm"
      >
        <div class="w-[420px] max-w-[90vw] rounded-lg border border-border bg-background p-4 shadow-2xl">
          <div class="mb-3 flex items-center gap-2">
            <AlertCircle class="size-5 text-yellow-500" />
            <h3 class="text-sm font-semibold">Agent 请求执行写操作</h3>
          </div>
          <div class="space-y-2 text-xs">
            <div>
              <span class="opacity-70">工具：</span>
              <span class="font-mono">{{ chat.pendingConfirmations[0]?.name }}</span>
            </div>
            <div>
              <div class="mb-1 opacity-70">参数：</div>
              <pre class="max-h-48 overflow-auto rounded bg-muted p-2 font-mono text-[11px] whitespace-pre-wrap break-all">{{ formatArgs(chat.pendingConfirmations[0]?.arguments || '') }}</pre>
            </div>
            <div class="rounded border border-yellow-500/30 bg-yellow-500/10 p-2 text-[11px] text-yellow-700 dark:text-yellow-300">
              此操作将修改本地文件或数据库。确认前请核对参数（尤其是路径）。
            </div>
          </div>
          <div class="mt-4 flex justify-end gap-2">
            <button
              class="rounded-md border border-border bg-background px-3 py-1.5 text-xs hover:bg-muted"
              @click="onRespond(false)"
            >
              拒绝
            </button>
            <button
              class="rounded-md bg-primary px-3 py-1.5 text-xs text-primary-foreground hover:opacity-90"
              @click="onRespond(true)"
            >
              确认执行
            </button>
          </div>
          <div class="mt-2 text-[10px] opacity-50">
            待确认：{{ chat.pendingConfirmations?.length || 0 }} 个
          </div>
        </div>
      </div>
    </Teleport>
  </aside>
</template>
