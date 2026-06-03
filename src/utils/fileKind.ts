import {
  FileText,
  FileCode,
  FileImage,
  FileSpreadsheet,
  Film,
  Music,
  Archive,
  File,
  Presentation,
  Mail,
  Globe,
  Database,
  type LucideIcon,
} from "@lucide/vue";

export function kindIcon(kind: string | null | undefined): LucideIcon {
  switch (kind) {
    case "pdf":
      return FileText;
    case "markdown":
    case "text":
      return FileText;
    case "code":
      return FileCode;
    case "html":
      return Globe;
    case "data":
      return Database;
    case "image":
      return FileImage;
    case "video":
      return Film;
    case "audio":
      return Music;
    case "spreadsheet":
      return FileSpreadsheet;
    case "presentation":
      return Presentation;
    case "document":
      return FileText;
    case "email":
      return Mail;
    case "archive":
      return Archive;
    default:
      return File;
  }
}

export function kindLabel(kind: string | null | undefined): string {
  switch (kind) {
    case "pdf":
      return "PDF";
    case "markdown":
      return "Markdown";
    case "text":
      return "文本";
    case "code":
      return "代码";
    case "html":
      return "网页";
    case "data":
      return "数据";
    case "image":
      return "图片";
    case "video":
      return "视频";
    case "audio":
      return "音频";
    case "spreadsheet":
      return "表格";
    case "presentation":
      return "演示";
    case "document":
      return "文档";
    case "email":
      return "邮件";
    case "archive":
      return "压缩包";
    default:
      return "其他";
  }
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

export function formatDate(ts: number): string {
  if (!ts) return "";
  const d = new Date(ts * 1000);
  const now = new Date();
  const sameDay =
    d.getFullYear() === now.getFullYear() &&
    d.getMonth() === now.getMonth() &&
    d.getDate() === now.getDate();
  if (sameDay) {
    return d.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" });
  }
  const sameYear = d.getFullYear() === now.getFullYear();
  return d.toLocaleDateString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    year: sameYear ? undefined : "2-digit",
  });
}

export function formatDateTime(ts: number): string {
  if (!ts) return "";
  return new Date(ts * 1000).toLocaleString("zh-CN");
}
