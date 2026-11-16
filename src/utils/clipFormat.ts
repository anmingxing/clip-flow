import type { ClipItem } from "../types/clip";

export function baseName(filePath: string): string {
  const normalized = filePath.replace(/[/\\]+$/, "");
  const parts = normalized.split(/[/\\]/);
  return parts[parts.length - 1] || filePath;
}

export function getPreviewTitle(item: ClipItem): string {
  switch (item.type) {
    case "text":
    case "richText":
    case "color": {
      const trimmed = item.text?.trim() ?? "";
      if (trimmed.length > 0) {
        const firstLine = trimmed.split(/\r?\n/)[0] ?? trimmed;
        return firstLine.slice(0, 120);
      }
      return "空白文本";
    }
    case "image": {
      if (item.imageWidth && item.imageHeight) {
        return `图片 (${item.imageWidth} × ${item.imageHeight})`;
      }
      return "图片";
    }
    case "fileURL": {
      const files = item.filePaths ?? [];
      if (files.length === 1) {
        return baseName(files[0]);
      }
      if (files.length > 1) {
        return `${baseName(files[0])} 等 ${files.length} 个文件`;
      }
      return "文件";
    }
  }
}

export function getPreviewSubtitle(item: ClipItem): string {
  const parts: string[] = [];
  if (item.sourceAppName) {
    parts.push(item.sourceAppName);
  }
  switch (item.type) {
    case "text":
    case "richText":
      if (item.text != null) {
        parts.push(`${item.text.length} 字符`);
      }
      break;
    case "image":
      parts.push("图像数据");
      break;
    case "fileURL":
      if (item.filePaths) {
        parts.push(`${item.filePaths.length} 项`);
      }
      break;
    case "color":
      if (item.text) {
        parts.push(`色值: ${item.text.trim()}`);
      }
      break;
  }
  return parts.join(" • ");
}

export function formatRelativeTime(isoString: string): string {
  const date = new Date(isoString);
  const diffSec = Math.floor((Date.now() - date.getTime()) / 1000);
  if (!Number.isFinite(diffSec) || diffSec < 10) {
    return "刚刚";
  }
  if (diffSec < 60) {
    return `${diffSec} 秒前`;
  }
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) {
    return `${diffMin} 分钟前`;
  }
  const diffHour = Math.floor(diffMin / 60);
  if (diffHour < 24) {
    return `${diffHour} 小时前`;
  }
  const diffDay = Math.floor(diffHour / 24);
  if (diffDay < 30) {
    return `${diffDay} 天前`;
  }
  return formatShortDate(isoString);
}

export function formatShortDate(isoString: string): string {
  const d = new Date(isoString);
  if (Number.isNaN(d.getTime())) return "";
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export function parseColorValues(raw: string): { hex: string; rgb: string } | null {
  const trimmed = raw.trim();
  const hexMatch = trimmed.match(/^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/);
  if (hexMatch) {
    const clean = hexMatch[1];
    let r = 0;
    let g = 0;
    let b = 0;
    if (clean.length === 3) {
      r = parseInt(clean[0] + clean[0], 16);
      g = parseInt(clean[1] + clean[1], 16);
      b = parseInt(clean[2] + clean[2], 16);
    } else {
      r = parseInt(clean.slice(0, 2), 16);
      g = parseInt(clean.slice(2, 4), 16);
      b = parseInt(clean.slice(4, 6), 16);
    }
    return {
      hex: trimmed.toUpperCase(),
      rgb: `rgb(${r}, ${g}, ${b})`,
    };
  }

  const rgbMatch = trimmed.match(
    /^rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*(?:,\s*(0|1|0?\.\d+)\s*)?\)$/i
  );
  if (rgbMatch) {
    const r = Math.min(255, Math.max(0, Number(rgbMatch[1])));
    const g = Math.min(255, Math.max(0, Number(rgbMatch[2])));
    const b = Math.min(255, Math.max(0, Number(rgbMatch[3])));
    const toHex = (n: number) => n.toString(16).padStart(2, "0").toUpperCase();
    return {
      hex: `#${toHex(r)}${toHex(g)}${toHex(b)}`,
      rgb: `rgb(${r}, ${g}, ${b})`,
    };
  }

  return null;
}

export function tryFormatJson(raw: string): { isJson: boolean; formatted: string } {
  const trimmed = raw.trim();
  if (
    !(trimmed.startsWith("{") && trimmed.endsWith("}")) &&
    !(trimmed.startsWith("[") && trimmed.endsWith("]"))
  ) {
    return { isJson: false, formatted: raw };
  }
  try {
    const parsed = JSON.parse(trimmed);
    return { isJson: true, formatted: JSON.stringify(parsed, null, 2) };
  } catch {
    return { isJson: false, formatted: raw };
  }
}

export function extractDetectedUrl(raw: string): { url: string; host: string } | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;

  const urlMatch = trimmed.match(/(https?:\/\/[^\s<>"'`]+|www\.[^\s<>"'`]+)/i);
  if (!urlMatch) return null;

  let candidate = urlMatch[0];
  if (candidate.toLowerCase().startsWith("www.")) {
    candidate = `https://${candidate}`;
  }

  try {
    const parsed = new URL(candidate);
    if (parsed.protocol === "http:" || parsed.protocol === "https:") {
      return { url: parsed.toString(), host: parsed.host || "网页链接" };
    }
  } catch {
    return null;
  }
  return null;
}
