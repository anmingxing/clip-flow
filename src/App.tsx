import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ClipboardListView } from "./components/ClipboardListView";
import {
  checkAccessibilityPermission,
  clearUnpinned,
  deleteItem,
  fetchHistory,
  hideWindow,
  isMacOS,
  onClipboardCleared,
  onClipboardUpdated,
  onWindowShown,
  pasteItem,
  requestAccessibilityPermission,
  togglePin,
} from "./services/tauriApi";
import { FILTER_TABS, type ClipContentType, type ClipItem, type FilterTab } from "./types/clip";

function filterToQueryParams(tab: FilterTab): {
  contentType: ClipContentType | null;
  pinnedOnly: boolean;
} {
  switch (tab) {
    case "all":
      return { contentType: null, pinnedOnly: false };
    case "pinned":
      return { contentType: null, pinnedOnly: true };
    case "text":
      return { contentType: "text", pinnedOnly: false };
    case "image":
      return { contentType: "image", pinnedOnly: false };
    case "fileURL":
      return { contentType: "fileURL", pinnedOnly: false };
    case "color":
      return { contentType: "color", pinnedOnly: false };
  }
}

export default function App() {
  const [items, setItems] = useState<ClipItem[]>([]);
  const [searchText, setSearchText] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [selectedFilter, setSelectedFilter] = useState<FilterTab>("all");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [hasAccessibilityPermission, setHasAccessibilityPermission] = useState(true);

  const searchInputRef = useRef<HTMLInputElement>(null);
  const isMac = useMemo(() => isMacOS(), []);

  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearch(searchText);
    }, 100);
    return () => clearTimeout(timer);
  }, [searchText]);

  const reloadItems = useCallback(
    async (resetIndex = false) => {
      const { contentType, pinnedOnly } = filterToQueryParams(selectedFilter);
      const fetched = await fetchHistory({
        query: debouncedSearch,
        contentType,
        pinnedOnly,
        limit: 100,
      });
      setItems(fetched);
      setSelectedIndex((prev) => {
        if (resetIndex || fetched.length === 0) return 0;
        return Math.min(prev, fetched.length - 1);
      });
    },
    [debouncedSearch, selectedFilter]
  );

  const refreshPermission = useCallback(async () => {
    const trusted = await checkAccessibilityPermission();
    setHasAccessibilityPermission(trusted);
  }, []);

  useEffect(() => {
    reloadItems(true);
  }, [reloadItems]);

  useEffect(() => {
    refreshPermission();
    searchInputRef.current?.focus();

    let unlistenUpdated: (() => void) | undefined;
    let unlistenCleared: (() => void) | undefined;
    let unlistenShown: (() => void) | undefined;

    onClipboardUpdated(() => {
      reloadItems(false);
    }).then((fn) => {
      unlistenUpdated = fn;
    });

    onClipboardCleared(() => {
      reloadItems(true);
    }).then((fn) => {
      unlistenCleared = fn;
    });

    onWindowShown(() => {
      refreshPermission();
      reloadItems(true);
      setTimeout(() => {
        searchInputRef.current?.focus();
      }, 30);
    }).then((fn) => {
      unlistenShown = fn;
    });

    return () => {
      unlistenUpdated?.();
      unlistenCleared?.();
      unlistenShown?.();
    };
  }, [reloadItems, refreshPermission]);

  const handlePaste = useCallback(async (item: ClipItem, plainTextOnly = false) => {
    await pasteItem(item.id, plainTextOnly, true);
  }, []);

  const handleTogglePinAt = useCallback(
    async (index: number) => {
      const target = items[index];
      if (!target) return;
      await togglePin(target.id);
      await reloadItems(false);
    },
    [items, reloadItems]
  );

  const handleDeleteAt = useCallback(
    async (index: number) => {
      const target = items[index];
      if (!target) return;
      await deleteItem(target.id);
      await reloadItems(false);
    },
    [items, reloadItems]
  );

  const handleClearUnpinned = useCallback(async () => {
    await clearUnpinned();
    await reloadItems(true);
  }, [reloadItems]);

  const cycleFilterTab = useCallback((forward: boolean) => {
    setSelectedFilter((current) => {
      const idx = FILTER_TABS.findIndex((t) => t.id === current);
      if (idx < 0) return "all";
      const nextIdx = forward
        ? (idx + 1) % FILTER_TABS.length
        : (idx - 1 + FILTER_TABS.length) % FILTER_TABS.length;
      return FILTER_TABS[nextIdx].id;
    });
    setSelectedIndex(0);
  }, []);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const primaryMod = isMac ? e.metaKey : e.ctrlKey;
      const keyLower = e.key.toLowerCase();

      // 1. Esc: clear search first, otherwise hide window
      if (e.key === "Escape") {
        e.preventDefault();
        if (searchText.length > 0) {
          setSearchText("");
        } else {
          hideWindow();
        }
        return;
      }

      // 2. Tab / Shift+Tab: cycle filter tabs
      if (e.key === "Tab") {
        e.preventDefault();
        cycleFilterTab(!e.shiftKey);
        return;
      }

      // 3. Move down: ArrowDown / Ctrl+N / Mod+J
      if (
        e.key === "ArrowDown" ||
        (e.ctrlKey && !e.metaKey && keyLower === "n") ||
        (primaryMod && keyLower === "j")
      ) {
        e.preventDefault();
        setSelectedIndex((prev) => (items.length > 0 ? Math.min(items.length - 1, prev + 1) : 0));
        return;
      }

      // 4. Move up: ArrowUp / Ctrl+P (macOS) / Mod+K
      if (
        e.key === "ArrowUp" ||
        (isMac && e.ctrlKey && !e.metaKey && keyLower === "p") ||
        (primaryMod && keyLower === "k")
      ) {
        e.preventDefault();
        setSelectedIndex((prev) => (items.length > 0 ? Math.max(0, prev - 1) : 0));
        return;
      }

      // 5. Enter / Shift+Enter: paste selected
      if (e.key === "Enter") {
        e.preventDefault();
        const target = items[selectedIndex];
        if (target) {
          handlePaste(target, e.shiftKey);
        }
        return;
      }

      // 6. Mod+Backspace: delete selected
      if (primaryMod && e.key === "Backspace") {
        e.preventDefault();
        handleDeleteAt(selectedIndex);
        return;
      }

      // 7. Mod+P: toggle pin
      if (primaryMod && !e.shiftKey && !e.altKey && keyLower === "p") {
        e.preventDefault();
        handleTogglePinAt(selectedIndex);
        return;
      }

      // 8. Mod+1..9: quick paste
      if (primaryMod && /^[1-9]$/.test(e.key)) {
        e.preventDefault();
        const quickIdx = Number(e.key) - 1;
        const target = items[quickIdx];
        if (target) {
          handlePaste(target, false);
        }
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    isMac,
    items,
    searchText,
    selectedIndex,
    cycleFilterTab,
    handleDeleteAt,
    handlePaste,
    handleTogglePinAt,
  ]);

  return (
    <ClipboardListView
      items={items}
      searchText={searchText}
      selectedFilter={selectedFilter}
      selectedIndex={selectedIndex}
      hasAccessibilityPermission={hasAccessibilityPermission}
      isMac={isMac}
      searchInputRef={searchInputRef}
      onSearchChange={setSearchText}
      onFilterChange={(tab) => {
        setSelectedFilter(tab);
        setSelectedIndex(0);
      }}
      onSelectIndex={setSelectedIndex}
      onPasteItem={(item) => handlePaste(item, false)}
      onTogglePin={handleTogglePinAt}
      onDeleteItem={handleDeleteAt}
      onClearUnpinned={handleClearUnpinned}
      onRequestPermission={() => {
        requestAccessibilityPermission();
      }}
    />
  );
}
