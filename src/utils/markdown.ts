import { marked } from "marked";
import DOMPurify from "dompurify";
import hljs from "highlight.js/lib/common";

marked.setOptions({
  gfm: true,
  breaks: true,
});

marked.use({
  renderer: {
    code({ text, lang }: { text: string; lang?: string }) {
      const language = lang && hljs.getLanguage(lang) ? lang : "plaintext";
      let highlighted: string;
      try {
        highlighted = hljs.highlight(text, { language, ignoreIllegals: true }).value;
      } catch {
        highlighted = escapeHtml(text);
      }
      const cls = lang ? `language-${lang}` : "language-plaintext";
      return `<pre class="rounded-md bg-zinc-900 p-3 text-xs overflow-x-auto"><code class="${cls} hljs">${highlighted}</code></pre>`;
    },
  },
});

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

export function renderMarkdown(text: string): string {
  if (!text) return "";
  const raw = marked.parse(text, { async: false }) as string;
  return DOMPurify.sanitize(raw, {
    ADD_ATTR: ["target"],
  });
}
