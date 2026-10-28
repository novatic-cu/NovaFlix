export type Screen = "search" | "browse" | "favorites" | "lists" | "downloads" | "settings";

export type HitKind = "file" | "dir";

export interface Hit {
  kind: HitKind;
  path: string;
  name: string;
  parent: string;
  section?: string | null;
  year?: number | null;
  ext?: string | null;
  size?: number | null;
  mtime?: string | null;
  score: number;
}

export interface SearchPage {
  hits: Hit[];
  total: number;
  took_ms: number;
}

export interface SearchFilter {
  section?: string;
  year?: number;
  kind?: "file" | "dir";
  ext?: string;
}

export interface Entry {
  name: string;
  path: string;
  kind: HitKind;
  ext?: string | null;
  size?: number | null;
  mtime?: string | null;
}

export interface Listing {
  path: string;
  entries: Entry[];
  fetched_at: string;
  cached: boolean;
}

export interface IndexStats {
  dirs: number;
  files: number;
  cache_bytes: number;
  cached_dirs: number;
  last_bootstrap?: string | null;
  last_crawl?: string | null;
  mode: string;
}

export interface Favorite {
  id: number;
  target_kind: "item" | "folder";
  path: string;
  label: string;
  note?: string | null;
  created_at: string;
}

export interface ListSummary {
  id: number;
  name: string;
  kind: "playlist" | "watchlist";
  count: number;
  created_at: string;
  updated_at: string;
}

export interface ListItem {
  path: string;
  name: string;
  position: number;
  added_at: string;
  watched_at?: string | null;
}

export interface Counter {
  path: string;
  attempts: number;
  completed: number;
  last_at?: string | null;
}

export interface DownloadRow {
  id: number;
  path: string;
  dest: string;
  bytes_total: number;
  bytes_done: number;
  state: "queued" | "active" | "paused" | "done" | "error" | "cancelled";
  started_at: string;
  finished_at?: string | null;
  error?: string | null;
}

export interface DownloadProgress {
  id: number;
  state: DownloadRow["state"];
  done: number;
  total: number;
  error?: string;
  dest?: string;
}

export interface AppInfo {
  version: string;
  data_dir: string;
  downloads_dir: string;
  base_url: string;
}

export interface CrawlProgress {
  dirs: number;
  queued: number;
  files: number;
  errors: number;
  max: number;
}
