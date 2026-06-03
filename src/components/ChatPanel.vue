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
} from "@lucide/vue";
import { renderMarkdown } from "../utils/markdown";
import "highlight.js/styles/github-dark.css";

const chat = useChatStore();
const input = ref("");
const scrollRef = useTemplateRef<HTMLDivElement>("scrollRef");
const inputRef = useTemplateRef<HTMLTextAreaElement>("inputRef");
const showHistory = ref(false);
const editingId = ref<number | null>(null);
const editingTitle = ref("");

onMounted(() => {
  chat.ensureListeners();
  chat.loadConversations();
});

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
    if (scrollRef.value) scrollRef.value.scrollTop = scrollRef.value.scrollHeight;
  },
);

function onSend() {
  const text = input.value;
  if (!text.trim()) return;
  input.value = "";
  chat.send(text);
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
          <input type="checkbox" v-model="chat.useRag" class="size-3" />
          RAG
        </label>
      </div>
      <div class="flex items-center gap-1">
        <button
          @click="showHistory = !showHistory"
          :class="[
            'rounded-md p-1 hover:bg-muted',
            showHistory ? 'text-primary bg-muted' : 'text-muted-foreground',
          ]"
          title="历史对话"
        >
          <History class="size-4" />
        </button>
        <button
          @click="chat.newConversation()"
          class="rounded-md p-1 text-muted-foreground hover:bg-muted"
          title="新对话"
        >
          <MessageSquarePlus class="size-4" />
        </button>
        <button
          @click="chat.togglePanel()"
          class="rounded-md p-1 text-muted-foreground hover:bg-muted"
          title="关闭"
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
                @keydown.enter="saveEdit"
                @keydown.escape="editingId = null"
                class="w-full rounded bg-background px-1 py-0.5 text-xs outline-none ring-1 ring-primary"
                autofocus
              />
              <button @click.stop="saveEdit" class="text-primary">
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
              @click.stop="startEdit(conv.id, conv.title)"
              class="rounded p-0.5 text-muted-foreground hover:bg-background hover:text-foreground"
              title="重命名"
            >
              <Pencil class="size-3" />
            </button>
            <button
              @click.stop="onDelete(conv.id)"
              class="rounded p-0.5 text-muted-foreground hover:bg-background hover:text-red-500"
              title="删除"
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

      <div ref="scrollRef" class="flex-1 overflow-auto p-4">
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
              <div v-else class="markdown-body" v-html="assistantHtml(msg.content)" />
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
                      @click="openFile(ref.path)"
                      class="rounded p-0.5 hover:bg-muted/80"
                      title="打开"
                    >
                      <ExternalLink class="size-3" />
                    </button>
                    <button
                      @click="revealInFinder(ref.path)"
                      class="rounded p-0.5 hover:bg-muted/80"
                      title="在 Finder 中显示"
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
      </div>
    </div>

    <div class="border-t border-border p-3">
      <div class="flex gap-2">
        <textarea
          ref="inputRef"
          v-model="input"
          @keydown="onKeydown"
          placeholder="问任何问题... (Enter 发送, Shift+Enter 换行)"
          rows="2"
          class="flex-1 resize-none rounded-md border border-border bg-background px-3 py-2 text-sm outline-none focus:border-primary"
        />
        <button
          @click="onSend"
          :disabled="!canSend"
          class="flex items-center justify-center rounded-md bg-primary px-3 text-primary-foreground hover:opacity-90 disabled:opacity-50"
        >
          <Loader2 v-if="chat.loading" class="size-4 animate-spin" />
          <Send v-else class="size-4" />
        </button>
      </div>
      <div class="mt-1.5 text-[10px] text-muted-foreground">
        <span v-if="chat.conversationId">对话 #{{ chat.conversationId }}</span>
        <span v-else>新对话</span>
      </div>
    </div>
  </aside>
</template>
