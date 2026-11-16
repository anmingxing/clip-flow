import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import type { ClipContentType, ClipItem } from "../types/clip";

const attachmentCache = new Map<string, string | null>();

export function isMacOS(): boolean {
  if (typeof navigator === "undefined") return false;
  return /mac/i.test(navigator.platform || navigator.userAgent);
}

export async function fetchHistory(params: {
  query?: string;
  contentType?: ClipContentType | null;
  pinnedOnly?: boolean;
  limit?: number;
}): Promise<ClipItem[]> {
  return invoke<ClipItem[]>("get_history", {
    query: params.query ?? null,
    contentType: params.contentType ?? null,
    pinnedOnly: params.pinnedOnly ?? false,
    limit: params.limit ?? 100,
  });
}

export async function getAttachmentBase64(fileName: string): Promise<string | null> {
  if (attachmentCache.has(fileName)) {
    return attachmentCache.get(fileName) ?? null;
  }
  const dataUrl = await invoke<string | null>("get_attachment_base64", { fileName });
  if (attachmentCache.size > 200) {
    const firstKey = attachmentCache.keys().next().value;
    if (firstKey !== undefined) {
      attachmentCache.delete(firstKey);
    }
  }
  attachmentCache.set(fileName, dataUrl);
  return dataUrl;
}

export async function pasteItem(
  id: string,
  plainTextOnly = false,
  autoPaste = true
): Promise<void> {
  await invoke("paste_item", { id, plainTextOnly, autoPaste });
}

export async function copyText(text: string): Promise<void> {
  await invoke("copy_text", { text });
}

export async function togglePin(id: string): Promise<boolean> {
  return invoke<boolean>("toggle_pin", { id });
}

export async function deleteItem(id: string): Promise<boolean> {
  return invoke<boolean>("delete_item", { id });
}

export async function clearUnpinned(): Promise<void> {
  await invoke("clear_unpinned");
}

export async function checkAccessibilityPermission(): Promise<boolean> {
  return invoke<boolean>("check_accessibility_permission");
}

export async function requestAccessibilityPermission(): Promise<boolean> {
  return invoke<boolean>("request_accessibility_permission");
}

export async function hideWindow(): Promise<void> {
  await invoke("hide_window");
}

export async function openExternalUrl(url: string): Promise<void> {
  await invoke("open_external_url", { url });
}

export function onClipboardUpdated(cb: (item: ClipItem) => void): Promise<UnlistenFn> {
  return listen<ClipItem>("clipboard://updated", (event) => cb(event.payload));
}

export function onClipboardCleared(cb: () => void): Promise<UnlistenFn> {
  return listen("clipboard://cleared", () => cb());
}

export function onWindowShown(cb: () => void): Promise<UnlistenFn> {
  return listen("window://shown", () => cb());
}
