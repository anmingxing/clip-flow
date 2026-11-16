import React from "react";
import clsx from "clsx";
import {
  AlertTriangle,
  FileText,
  Folder,
  Image as ImageIcon,
  Inbox,
  Layers,
  Palette,
  Pin,
  Search,
  Trash2,
  XCircle,
} from "lucide-react";
import { FILTER_TABS, type ClipItem, type FilterTab } from "../types/clip";
import { ClipItemRow } from "./ClipItemRow";
import { ClipDetailPreview } from "./ClipDetailPreview";

interface ClipboardListViewProps {
  items: ClipItem[];
  searchText: string;
  selectedFilter: FilterTab;
  selectedIndex: number;
  hasAccessibilityPermission: boolean;
  isMac: boolean;
  searchInputRef: React.RefObject<HTMLInputElement>;
  onSearchChange: (value: string) => void;
  onFilterChange: (tab: FilterTab) => void;
  onSelectIndex: (index: number) => void;
  onPasteItem: (item: ClipItem) => void;
  onTogglePin: (index: number) => void;
  onDeleteItem: (index: number) => void;
  onClearUnpinned: () => void;
  onRequestPermission: () => void;
}

export const ClipboardListView: React.FC<ClipboardListViewProps> = ({
  items,
  searchText,
  selectedFilter,
  selectedIndex,
  hasAccessibilityPermission,
  isMac,
  searchInputRef,
  onSearchChange,
  onFilterChange,
  onSelectIndex,
  onPasteItem,
  onTogglePin,
  onDeleteItem,
  onClearUnpinned,
  onRequestPermission,
}) => {
  const selectedItem = items[selectedIndex] ?? null;
  const modSymbol = isMac ? "⌘" : "Ctrl+";

  const renderTabIcon = (tab: FilterTab) => {
    switch (tab) {
      case "all":
        return <Layers className="w-3 h-3" />;
      case "pinned":
        return <Pin className="w-3 h-3" />;
      case "text":
        return <FileText className="w-3 h-3" />;
      case "image":
        return <ImageIcon className="w-3 h-3" />;
      case "fileURL":
        return <Folder className="w-3 h-3" />;
      case "color":
        return <Palette className="w-3 h-3" />;
    }
  };

  return (
    <div className="w-[860px] h-[540px] flex rounded-2xl overflow-hidden backdrop-blur-2xl bg-white/90 dark:bg-[#1a1d28]/92 border border-black/15 dark:border-white/15 shadow-2xl text-slate-900 dark:text-slate-100">
      {/* Left Column: Search + Tabs + List + Footer */}
      <div className="w-[480px] h-full flex flex-col bg-black/[0.015] dark:bg-white/[0.015]">
        {/* Header Section */}
        <div
          data-tauri-drag-region
          className="px-3.5 pt-3.5 pb-2.5 flex flex-col gap-2.5 border-b border-black/[0.08] dark:border-white/[0.08]"
        >
          {/* Search Input */}
          <div className="flex items-center gap-2 px-2.5 py-[6.5px] rounded-lg bg-white/75 dark:bg-black/30 border border-black/10 dark:border-white/15 focus-within:border-[#0A84FF]/70 focus-within:ring-1 focus-within:ring-[#0A84FF]/30 transition">
            <Search className="w-3.5 h-3.5 text-slate-400 shrink-0" />
            <input
              ref={searchInputRef}
              type="text"
              value={searchText}
              onChange={(e) => onSearchChange(e.target.value)}
              placeholder="搜索剪切板历史..."
              className="flex-1 bg-transparent text-[13px] outline-none placeholder:text-slate-400 text-slate-900 dark:text-slate-100"
            />
            {searchText.length > 0 && (
              <button
                type="button"
                onClick={() => onSearchChange("")}
                title="清空搜索词 (Esc)"
                className="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200"
              >
                <XCircle className="w-3.5 h-3.5" />
              </button>
            )}
          </div>

          {/* Filter Capsule Bar */}
          <div className="flex items-center gap-1.5 overflow-x-auto no-scrollbar">
            {FILTER_TABS.map((tab) => {
              const active = selectedFilter === tab.id;
              return (
                <button
                  key={tab.id}
                  type="button"
                  onClick={() => onFilterChange(tab.id)}
                  className={clsx(
                    "inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-[11px] shrink-0 transition-colors",
                    active
                      ? "bg-[#0A84FF] text-white font-semibold shadow-sm"
                      : "bg-black/[0.05] dark:bg-white/[0.07] text-slate-700 dark:text-slate-200/85 hover:bg-black/[0.09] dark:hover:bg-white/[0.12]"
                  )}
                >
                  {renderTabIcon(tab.id)}
                  <span>{tab.title}</span>
                </button>
              );
            })}
          </div>
        </div>

        {/* Accessibility Warning Banner (macOS) */}
        {!hasAccessibilityPermission && (
          <div className="flex items-center gap-2 px-3.5 py-1.5 bg-orange-500/15 border-b border-orange-500/20">
            <AlertTriangle className="w-3.5 h-3.5 text-orange-500 shrink-0" />
            <span className="text-[11px] text-slate-600 dark:text-slate-300 truncate">
              缺少辅助功能权限，无法自动模拟 Cmd+V 粘贴
            </span>
            <button
              type="button"
              onClick={onRequestPermission}
              className="ml-auto px-2 py-0.5 rounded bg-orange-500 hover:bg-orange-600 text-white text-[10.5px] font-medium shrink-0"
            >
              去授权
            </button>
          </div>
        )}

        {/* History List or Empty State */}
        {items.length === 0 ? (
          <div className="flex-1 flex flex-col items-center justify-center p-6 text-center select-none">
            <div className="w-[72px] h-[72px] rounded-full bg-black/[0.03] dark:bg-white/[0.04] flex items-center justify-center mb-3.5">
              <Inbox className="w-8 h-8 text-slate-400/60" />
            </div>
            <p className="text-[13px] font-semibold text-slate-700 dark:text-slate-200">
              {searchText ? "没有找到匹配的记录" : "剪切板历史为空"}
            </p>
            <p className="text-[11px] text-slate-500 dark:text-slate-400 mt-1">
              {searchText
                ? "尝试更换搜索关键词或切换顶部分类"
                : "复制任意文本、代码、图片或链接将自动沉淀在此"}
            </p>
          </div>
        ) : (
          <div className="flex-1 overflow-y-auto no-scrollbar px-2.5 py-2 flex flex-col gap-1">
            {items.map((item, idx) => (
              <ClipItemRow
                key={item.id}
                item={item}
                index={idx}
                isSelected={selectedIndex === idx}
                isMac={isMac}
                onSelect={() => onSelectIndex(idx)}
                onPaste={() => onPasteItem(item)}
                onTogglePin={() => onTogglePin(idx)}
                onDelete={() => onDeleteItem(idx)}
              />
            ))}
          </div>
        )}

        {/* Footer Section */}
        <div className="px-3.5 py-2 border-t border-black/[0.08] dark:border-white/[0.08] flex items-center gap-2">
          <KeyHint kbd="⏎" label="粘贴" />
          <KeyHint kbd="⇧⏎" label="纯文本" />
          <KeyHint kbd="Tab" label="标签" />
          <KeyHint kbd={`${modSymbol}P`} label="置顶" />
          <KeyHint kbd={`${modSymbol}⌫`} label="删除" />
          <KeyHint kbd="Esc" label="退出" />

          <button
            type="button"
            onClick={onClearUnpinned}
            title="一键抹除所有未置顶的历史记录"
            className="ml-auto inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-black/[0.04] dark:bg-white/[0.05] hover:bg-black/[0.08] dark:hover:bg-white/[0.1] text-slate-500 dark:text-slate-400 text-[10px] font-medium shrink-0 transition-colors"
          >
            <Trash2 className="w-2.5 h-2.5" />
            <span>清空未置顶</span>
          </button>
        </div>
      </div>

      {/* Divider */}
      <div className="w-px h-full bg-black/10 dark:bg-white/10 shrink-0" />

      {/* Right Column: Detail Preview */}
      <div className="w-[379px] h-full flex bg-black/[0.02] dark:bg-white/[0.02]">
        <ClipDetailPreview item={selectedItem} />
      </div>
    </div>
  );
};

const KeyHint: React.FC<{ kbd: string; label: string }> = ({ kbd, label }) => (
  <span className="inline-flex items-center gap-1 shrink-0">
    <kbd className="px-1 py-[1.5px] rounded-[3.5px] text-[9.5px] font-bold font-mono bg-black/[0.06] dark:bg-white/[0.08] border border-black/10 dark:border-white/15 shadow-sm">
      {kbd}
    </kbd>
    <span className="text-[10.5px] text-slate-500 dark:text-slate-400">{label}</span>
  </span>
);
