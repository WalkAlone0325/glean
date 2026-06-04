import { useToastStore } from "../stores/toast";

export type ErrorKind = "api_key_missing" | "network" | "rate_limit" | "database" | "unknown";

export interface ClassifiedError {
  kind: ErrorKind;
  message: string;
  canOpenSettings: boolean;
}

export function classifyError(raw: unknown): ClassifiedError {
  const msg = String(raw).toLowerCase();

  if (msg.includes("api key") || msg.includes("请先填入") || msg.includes("unauthorized") || msg.includes("401")) {
    return {
      kind: "api_key_missing",
      message: "API Key 无效或未配置",
      canOpenSettings: true,
    };
  }
  if (msg.includes("rate limit") || msg.includes("429") || msg.includes("too many")) {
    return {
      kind: "rate_limit",
      message: "请求过于频繁，请稍后重试",
      canOpenSettings: false,
    };
  }
  if (msg.includes("network") || msg.includes("failed to connect") || msg.includes("timeout") || msg.includes("dns")) {
    return {
      kind: "network",
      message: "网络连接失败，请检查网络或代理",
      canOpenSettings: true,
    };
  }
  if (msg.includes("sqlite") || msg.includes("database") || msg.includes("db mutex")) {
    return {
      kind: "database",
      message: "数据库错误",
      canOpenSettings: false,
    };
  }
  return {
    kind: "unknown",
    message: String(raw),
    canOpenSettings: false,
  };
}

export function reportError(raw: unknown, onOpenSettings?: () => void) {
  const toast = useToastStore();
  const err = classifyError(raw);
  const action = err.canOpenSettings && onOpenSettings
    ? { label: "打开设置", onClick: onOpenSettings }
    : undefined;
  toast.push(err.message, "error", { action, durationMs: 5000 });
}

export function reportSuccess(message: string) {
  const toast = useToastStore();
  toast.success(message);
}
