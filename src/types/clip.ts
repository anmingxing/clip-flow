export type ClipContentType = "text" | "richText" | "image" | "fileURL" | "color";

export interface ClipItem {
  id: string;
  type: ClipContentType;
  text?: string | null;
  richTextData?: string | null;
  imageFileName?: string | null;
  imageWidth?: number | null;
  imageHeight?: number | null;
  filePaths?: string[] | null;
  sourceAppBundleId?: string | null;
  sourceAppName?: string | null;
  createdAt: string;
  isPinned: boolean;
  contentHash: string;
}

export type FilterTab = "all" | "pinned" | "text" | "image" | "fileURL" | "color";

export interface FilterTabConfig {
  id: FilterTab;
  title: string;
}

export const FILTER_TABS: FilterTabConfig[] = [
  { id: "all", title: "全部" },
  { id: "pinned", title: "已置顶" },
  { id: "text", title: "文本" },
  { id: "image", title: "图片" },
  { id: "fileURL", title: "文件" },
  { id: "color", title: "颜色" },
];

export const CONTENT_TYPE_LABEL: Record<ClipContentType, string> = {
  text: "文本",
  richText: "富文本",
  image: "图片",
  fileURL: "文件",
  color: "颜色",
};
