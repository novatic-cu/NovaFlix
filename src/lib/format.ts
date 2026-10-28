export function formatBytes(value?: number | null): string {
  if (value === undefined || value === null || value <= 0) return "—";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let size = value;
  let unit = 0;
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024;
    unit += 1;
  }
  return `${size >= 10 || unit === 0 ? Math.round(size) : size.toFixed(1)} ${units[unit]}`;
}

export function formatDate(value?: string | null): string {
  if (!value) return "—";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleDateString("es-ES", {
    day: "2-digit",
    month: "short",
    year: "numeric",
  });
}

export function decodePath(path: string): string {
  try {
    return decodeURIComponent(path);
  } catch {
    return path;
  }
}

export function absoluteUrl(base: string, path: string): string {
  const normalized = path.startsWith("/") ? path : `/${path}`;
  return `${base}${normalized.replaceAll(" ", "%20")}`;
}

export function isVideo(ext?: string | null): boolean {
  if (!ext) return false;
  return ["mkv", "mp4", "avi", "webm", "mov", "m4v", "ts", "mpg", "mpeg"].includes(
    ext.toLowerCase(),
  );
}

export function basename(path: string): string {
  const trimmed = path.replace(/\/+$/, "");
  return trimmed.split("/").pop() ?? trimmed;
}

export function folderOf(path: string): string {
  const trimmed = path.replace(/\/+$/, "");
  const index = trimmed.lastIndexOf("/");
  return index <= 0 ? "/" : trimmed.slice(0, index);
}

export function breadcrumbs(path: string): { label: string; path: string }[] {
  const parts = path.replace(/\/+$/, "").split("/").filter(Boolean);
  const crumbs = [{ label: "Raíz", path: "/" }];
  let current = "";
  for (const part of parts) {
    current += `/${part}`;
    crumbs.push({ label: decodePath(part), path: `${current}/` });
  }
  return crumbs;
}

export const VIDEO_EXTS = ["mkv", "mp4", "avi", "webm", "mov", "m4v"] as const;
export const SUB_EXTS = ["srt", "sub", "ass", "vtt"] as const;
