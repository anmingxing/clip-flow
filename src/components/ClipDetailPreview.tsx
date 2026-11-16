import React, { useEffect, useState } from "react";
import {
  AlignLeft,
  AppWindow,
  ArrowUpRight,
  Braces,
  Check,
  Copy,
  FileText,
  Folder,
  Globe,
  Image as ImageIcon,
  Palette,
  PanelRight,
  Pin,
} from "lucide-react";
import { CONTENT_TYPE_LABEL, type ClipContentType, type ClipItem } from "../types/clip";
import { copyText, getAttachmentBase64, openExternalUrl } from "../services/tauriApi";
import {
  baseName,
  extractDetectedUrl,
  formatShortDate,
  parseColorValues,
  tryFormatJson,
} from "../utils/clipFormat";

interface ClipDetailPreviewProps {
  item: ClipItem | null;
}

export const ClipDetailPreview: React.FC<ClipDetailPreviewProps> = ({ item }) => {
  const [imageDataUrl, setImageDataUrl] = useState<string | null>(null);
  const [copiedValue, setCopiedValue] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    if (item?.type === "image" && item.imageFileName) {
      getAttachmentBase64(item.imageFileName).then((dataUrl) => {
        if (active) setImageDataUrl(dataUrl);
      });
    } else {
      setImageDataUrl(null);
    }
    return () => {
      active = false;
    };
  }, [item?.id, item?.type, item?.imageFileName]);

  const handleCopyColorValue = async (value: string) => {
    await copyText(value);
    setCopiedValue(value);
    setTimeout(() => {
      setCopiedValue((prev) => (prev === value ? null : prev));
    }, 1200);
  };

  if (!item) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center p-6 text-center select-none">
        <div className="w-16 h-16 rounded-full bg-black/[0.04] dark:bg-white/[0.04] flex items-center justify-center mb-3">
          <PanelRight className="w-7 h-7 text-slate-400/60" />
        </div>
        <p className="text-[13px] font-medium text-slate-700 dark:text-slate-200">暂无选中内容</p>
        <p className="text-[11px] text-slate-500 dark:text-slate-400 mt-0.5">
          在左侧列表中单击以查看详情
        </p>
      </div>
    );
  }

  const renderTypeIcon = (type: ClipContentType) => {
    switch (type) {
      case "text":
        return <FileText className="w-3 h-3" />;
      case "richText":
        return <AlignLeft className="w-3 h-3" />;
      case "image":
        return <ImageIcon className="w-3 h-3" />;
      case "fileURL":
        return <Folder className="w-3 h-3" />;
      case "color":
        return <Palette className="w-3 h-3" />;
    }
  };

  const renderMetaStat = () => {
    switch (item.type) {
      case "text":
      case "richText": {
        const text = item.text ?? "";
        const lines = text.length > 0 ? text.split(/\r?\n/).length : 0;
        return `${text.length} 字符 • ${lines} 行`;
      }
      case "image":
        if (item.imageWidth && item.imageHeight) {
          return `${item.imageWidth} × ${item.imageHeight} 像素`;
        }
        return null;
      case "fileURL":
        if (item.filePaths) {
          return `${item.filePaths.length} 个文件`;
        }
        return null;
      case "color":
        return null;
    }
  };

  const renderBody = () => {
    if (item.type === "color") {
      const colorRaw = item.text?.trim() || "#000000";
      const parsed = parseColorValues(colorRaw);
      return (
        <div className="flex flex-col gap-3.5">
          <div
            className="h-[120px] rounded-[10px] border border-white/20 shadow-md"
            style={{ backgroundColor: colorRaw }}
          />
          <div className="flex flex-col gap-2">
            <ColorRow
              label="HEX"
              value={parsed?.hex ?? colorRaw.toUpperCase()}
              copied={copiedValue === (parsed?.hex ?? colorRaw.toUpperCase())}
              onCopy={() => handleCopyColorValue(parsed?.hex ?? colorRaw.toUpperCase())}
            />
            {parsed && (
              <ColorRow
                label="RGB"
                value={parsed.rgb}
                copied={copiedValue === parsed.rgb}
                onCopy={() => handleCopyColorValue(parsed.rgb)}
              />
            )}
          </div>
        </div>
      );
    }

    if (item.type === "image") {
      return (
        <div className="flex flex-col items-center justify-center">
          {imageDataUrl ? (
            <img
              src={imageDataUrl}
              alt="clipboard preview"
              className="max-w-full rounded-lg shadow-md object-contain"
            />
          ) : (
            <p className="text-xs text-slate-500 dark:text-slate-400">无法加载图片文件</p>
          )}
        </div>
      );
    }

    if (item.type === "fileURL") {
      const files = item.filePaths ?? [];
      return (
        <div className="flex flex-col gap-2">
          {files.map((path, idx) => (
            <div
              key={`${path}-${idx}`}
              className="flex items-center gap-2.5 p-2 rounded-md bg-black/[0.04] dark:bg-white/[0.05]"
            >
              <div className="w-7 h-7 rounded bg-amber-500/15 text-amber-500 flex items-center justify-center shrink-0">
                <Folder className="w-4 h-4" />
              </div>
              <div className="min-w-0 flex-1">
                <p className="text-xs font-medium text-slate-800 dark:text-slate-100 truncate">
                  {baseName(path)}
                </p>
                <p className="text-[10px] text-slate-500 dark:text-slate-400 truncate">{path}</p>
              </div>
            </div>
          ))}
        </div>
      );
    }

    const rawText = item.text ?? "";
    const detectedUrl = extractDetectedUrl(rawText);
    const { isJson, formatted } = tryFormatJson(rawText);

    return (
      <div className="flex flex-col gap-2.5">
        {detectedUrl && (
          <div
            onClick={() => openExternalUrl(detectedUrl.url)}
            className="flex items-center gap-2.5 p-2.5 rounded-[9px] bg-blue-500/[0.08] border border-blue-500/25 cursor-pointer hover:bg-blue-500/[0.12] transition-colors"
          >
            <div className="w-8 h-8 rounded-full bg-blue-500/15 text-blue-500 flex items-center justify-center shrink-0">
              <Globe className="w-4 h-4" />
            </div>
            <div className="min-w-0 flex-1">
              <p className="text-[11.5px] font-semibold text-slate-800 dark:text-slate-100 truncate">
                {detectedUrl.host}
              </p>
              <p className="text-[10px] text-blue-500 truncate">{detectedUrl.url}</p>
            </div>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                openExternalUrl(detectedUrl.url);
              }}
              className="flex items-center gap-1 px-2 py-1 rounded-md bg-blue-500/15 border border-blue-500/30 text-blue-500 text-[10.5px] font-semibold shrink-0 hover:bg-blue-500/25"
            >
              <span>打开网址</span>
              <ArrowUpRight className="w-3 h-3" />
            </button>
          </div>
        )}

        {isJson && (
          <div className="flex items-center gap-1.5 px-2 py-1 rounded-[5px] bg-emerald-500/15 text-emerald-500 w-fit">
            <Braces className="w-3 h-3" />
            <span className="text-[10px] font-semibold font-mono">JSON 格式化数据</span>
          </div>
        )}

        <div className="p-2.5 rounded-lg bg-white/70 dark:bg-black/25 border border-black/[0.08] dark:border-white/[0.08]">
          <pre className="text-[11.5px] leading-[1.55] font-mono text-slate-800 dark:text-slate-100/90 whitespace-pre-wrap break-words select-text m-0">
            {formatted}
          </pre>
        </div>
      </div>
    );
  };

  const metaStat = renderMetaStat();

  return (
    <div className="flex flex-col h-full w-full">
      <div className="px-3.5 pt-3.5 pb-2.5 flex flex-col gap-2 border-b border-black/[0.08] dark:border-white/[0.08]">
        <div className="flex items-center gap-1.5">
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-blue-500/15 text-blue-500 text-[11px] font-semibold">
            {renderTypeIcon(item.type)}
            {CONTENT_TYPE_LABEL[item.type]}
          </span>

          {item.isPinned && (
            <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded-full bg-orange-500/15 text-orange-500 text-[10px] font-medium">
              <Pin className="w-2.5 h-2.5" />
              置顶
            </span>
          )}

          <span className="ml-auto text-[10.5px] text-slate-400">
            {formatShortDate(item.createdAt)}
          </span>
        </div>

        {(item.sourceAppName || metaStat) && (
          <div className="flex items-center gap-2.5 text-[10.5px] text-slate-500 dark:text-slate-400">
            {item.sourceAppName && (
              <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-black/[0.04] dark:bg-white/[0.06] font-medium">
                <AppWindow className="w-2.5 h-2.5" />
                {item.sourceAppName}
              </span>
            )}
            {metaStat && <span>{metaStat}</span>}
          </div>
        )}
      </div>

      <div className="flex-1 overflow-y-auto no-scrollbar p-3.5">{renderBody()}</div>
    </div>
  );
};

const ColorRow: React.FC<{
  label: string;
  value: string;
  copied: boolean;
  onCopy: () => void;
}> = ({ label, value, copied, onCopy }) => (
  <div className="flex items-center px-2.5 py-1.5 rounded-md bg-black/[0.04] dark:bg-white/[0.05]">
    <span className="w-9 text-[11px] font-bold font-mono text-slate-500 dark:text-slate-400">
      {label}
    </span>
    <span className="text-xs font-mono text-slate-800 dark:text-slate-100 select-text">
      {value}
    </span>
    <button
      type="button"
      onClick={onCopy}
      title={`拷贝 ${value}`}
      className="ml-auto text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 transition-colors"
    >
      {copied ? <Check className="w-3.5 h-3.5 text-emerald-500" /> : <Copy className="w-3.5 h-3.5" />}
    </button>
  </div>
);
