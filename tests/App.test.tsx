import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import App from "../src/App";

const api = vi.hoisted(() => ({
  fetchHistory: vi.fn(),
  pasteItem: vi.fn(),
  onClipboardError: vi.fn(),
}));

vi.mock("../src/services/tauriApi", () => ({
  ...api,
  isMacOS: () => true,
  checkAccessibilityPermission: async () => true,
  onClipboardUpdated: async () => () => {},
  onClipboardCleared: async () => () => {},
  onWindowShown: async () => () => {},
  togglePin: vi.fn(),
  deleteItem: vi.fn(),
  clearUnpinned: vi.fn(),
  hideWindow: vi.fn(),
  requestAccessibilityPermission: vi.fn(),
  getAttachmentBase64: vi.fn(),
  copyText: vi.fn(),
  openExternalUrl: vi.fn(),
}));

beforeEach(() => {
  vi.clearAllMocks();
  Element.prototype.scrollIntoView = vi.fn();
  api.fetchHistory.mockResolvedValue([{
    id: "one", type: "text", text: "hello", isPinned: false,
    createdAt: "2026-11-16T00:00:00Z", contentHash: "hash",
  }]);
  api.pasteItem.mockResolvedValue(undefined);
  api.onClipboardError.mockResolvedValue(() => {});
});

afterEach(cleanup);

async function openHistory() {
  render(<App />);
  await screen.findByText("hello", { selector: "pre" });
  return screen.getByPlaceholderText("搜索剪切板历史...");
}

describe("keyboard paste", () => {
  it("leaves Enter to the Chinese IME while composing", async () => {
    const input = await openHistory();
    fireEvent.keyDown(input, { key: "Enter", isComposing: true });
    expect(api.pasteItem).not.toHaveBeenCalled();
  });

  it("handles Safari's final composition Enter with keyCode 229", async () => {
    const input = await openHistory();
    fireEvent.keyDown(input, { key: "Enter", isComposing: false, keyCode: 229 });
    expect(api.pasteItem).not.toHaveBeenCalled();
  });

  it.each([false, true])("preserves normal paste (plain text: %s)", async (shiftKey) => {
    const input = await openHistory();
    fireEvent.keyDown(input, { key: "Enter", shiftKey });
    await waitFor(() => expect(api.pasteItem).toHaveBeenCalledWith("one", shiftKey, true));
  });

  it("displays IPC paste errors", async () => {
    const input = await openHistory();
    api.pasteItem.mockRejectedValueOnce("无法激活目标应用");
    fireEvent.keyDown(input, { key: "Enter" });
    expect((await screen.findByRole("alert")).textContent).toContain("无法激活目标应用");
  });

  it("displays background storage errors", async () => {
    await openHistory();
    const reportError = api.onClipboardError.mock.calls.at(-1)?.[0];
    reportError("Cannot save history");
    expect((await screen.findByRole("alert")).textContent).toContain("Cannot save history");
    fireEvent.click(screen.getByRole("button", { name: "关闭错误提示" }));
    expect(screen.queryByRole("alert")).toBeNull();
  });
});
