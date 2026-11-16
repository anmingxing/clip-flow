import React, { useEffect, useRef, useState } from "react";
import clsx from "clsx";
import { FileText, Folder, Image as ImageIcon, Pin, PinOff, Trash2, AlignLeft } from "lucide-react";
import type { ClipItem } from "../types/clip";
import { getAttachmentBase64 } from "../services/tauriApi";
import { formatRelativeTime, getPreviewSubtitle, getPreviewTitle } from "../utils/clipFormat";

interface ClipItemRowProps {
  item: ClipItem;
  index: number;
  isSelected: boolean;
  isMac: boolean;
  onSelect: () => void;
  onPaste: () => void;
  onTogglePin: () => void;
  onDelete: () => void;
}

export const ClipItemRow: React.FC<ClipItemRowProps> = ({
  item,
  index,
  isSelected,
  isMac,
  onSelect,
  onPaste,
  onTogglePin,
  onDelete,
}) => {
  const rowRef = useRef<HTMLDivElement>(null);
  const [thumbUrl, setThumbUrl] = useState<string | null>(null);

  useEffect(() => {
    if (isSelected && rowRef.current) {
      rowRef.current.scrollIntoView({ block: "nearest", behavior: "smooth" });
    }
  }, [isSelected]);

  useEffect(() => {
    let active = true;
    if (item.type === "image" && item.imageFileName) {
      getAttachmentBase64(item.imageFileName).then((dataUrl) => {
        if (active) setThumbUrl(dataUrl);
      });
    } else {
      setThumbUrl(null);
    }
    return () => {
      active = false;
    };
  }, [item.type, item.imageFileName]);

  const title = getPreviewTitle(item);
  const subtitle = `${getPreviewSubtitle(item)} • ${formatRelativeTime(item.createdAt)}`;
  const modSymbol = isMac ? "⌘" : "^";

  const renderLeadingMedia = () => {
    if (item.type === "color" && item.text) {
      return (
        <div
          className="w-[26px] h-[26px] rounded-md shrink-0 border border-white/25 shadow-sm"
          style={{ backgroundColor: item.text.trim() }}
        />
      );
    }

    if (item.type === "image") {
      if (thumbUrl) {
        return (
          <img
            src={thumbUrl}
            alt="thumbnail"
            className="w-[26px] h-[26px] rounded-md object-cover shrink-0 border border-black/10 dark:border-white/10"
          />
        );
      }
      return (
        <SemanticIcon isSelected={isSelected} colorClass="text-emerald-500 bg-emerald-500/15">
          <ImageIcon className="w-3.5 h-3.5" />
        </SemanticIcon>
      );
    }

    if (item.type === "fileURL") {
      return (
        <SemanticIcon isSelected={isSelected} colorClass="text-amber-500 bg-amber-500/15">
          <Folder className="w-3.5 h-3.5" />
        </SemanticIcon>
      );
    }

    if (item.type === "richText") {
      return (
        <SemanticIcon isSelected={isSelected} colorClass="text-purple-500 bg-purple-500/15">
          <AlignLeft className="w-3.5 h-3.5" />
        </SemanticIcon>
      );
    }

    return (
      <SemanticIcon isSelected={isSelected} colorClass="text-blue-500 bg-blue-500/15">
        <FileText className="w-3.5 h-3.5" />
      </SemanticIcon>
    );
  };

  return (
    <div
      ref={rowRef}
      onClick={onSelect}
      onDoubleClick={() => {
        onSelect();
        onPaste();
      }}
      className={clsx(
        "group flex items-center gap-2.5 px-2.5 py-[7px] rounded-lg cursor-default transition-colors duration-100",
        isSelected
          ? "bg-[#0A84FF] text-white"
          : "hover:bg-black/[0.05] dark:hover:bg-white/[0.06] text-slate-900 dark:text-slate-100"
      )}
    >
      {index < 9 ? (
        <span
          className={clsx(
            "w-7 h-5 rounded flex items-center justify-center text-[11px] font-semibold font-mono shrink-0",
            isSelected
              ? "bg-white/20 text-white/95"
              : "bg-black/[0.06] dark:bg-white/[0.08] text-slate-500 dark:text-slate-400"
          )}
        >
          {modSymbol}
          {index + 1}
        </span>
      ) : (
        <span className="w-7 shrink-0" />
      )}

      {renderLeadingMedia()}

      <div className="min-w-0 flex-1 flex flex-col gap-0.5">
        <div className="flex items-center gap-1.5 min-w-0">
          <span className="text-[13px] font-medium truncate">{title}</span>
          {item.isPinned && (
            <Pin
              className={clsx(
                "w-3 h-3 shrink-0",
                isSelected ? "text-amber-300" : "text-orange-500"
              )}
            />
          )}
        </div>
        <span
          className={clsx(
            "text-[11px] truncate",
            isSelected ? "text-white/80" : "text-slate-500 dark:text-slate-400"
          )}
        >
          {subtitle}
        </span>
      </div>

      <div
        className={clsx(
          "flex items-center gap-1.5 shrink-0 transition-opacity",
          isSelected ? "opacity-100" : "opacity-0 group-hover:opacity-100"
        )}
      >
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onTogglePin();
          }}
          title={item.isPinned ? `取消置顶 (${modSymbol}P)` : `置顶 (${modSymbol}P)`}
          className={clsx(
            "w-6 h-6 rounded-full flex items-center justify-center transition-colors",
            isSelected
              ? "bg-white/20 hover:bg-white/30 text-white"
              : "bg-black/[0.06] dark:bg-white/[0.08] hover:bg-black/10 dark:hover:bg-white/15 text-slate-500 dark:text-slate-300"
          )}
        >
          {item.isPinned ? <PinOff className="w-3 h-3" /> : <Pin className="w-3 h-3" />}
        </button>

        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onDelete();
          }}
          title={`删除此记录 (${modSymbol}⌫)`}
          className={clsx(
            "w-6 h-6 rounded-full flex items-center justify-center transition-colors",
            isSelected
              ? "bg-white/20 hover:bg-white/30 text-white"
              : "bg-black/[0.06] dark:bg-white/[0.08] hover:bg-black/10 dark:hover:bg-white/15 text-slate-500 dark:text-slate-300"
          )}
        >
          <Trash2 className="w-3 h-3" />
        </button>
      </div>
    </div>
  );
};

const SemanticIcon: React.FC<{
  isSelected: boolean;
  colorClass: string;
  children: React.ReactNode;
}> = ({ isSelected, colorClass, children }) => (
  <div
    className={clsx(
      "w-[26px] h-[26px] rounded-md flex items-center justify-center shrink-0",
      isSelected ? "bg-white/20 text-white" : colorClass
    )}
  >
    {children}
  </div>
);
