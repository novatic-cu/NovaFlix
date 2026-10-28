import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  Counter,
  DownloadRow,
  Favorite,
  IndexStats,
  Listing,
  ListItem,
  ListSummary,
  SearchFilter,
  SearchPage,
} from "./types";

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),

  indexStats: () => invoke<IndexStats>("index_stats"),
  indexBootstrap: (force = false) => invoke<IndexStats>("index_bootstrap", { force }),
  crawlStart: (maxDirs?: number) => invoke<void>("crawl_start", { maxDirs: maxDirs ?? null }),
  crawlStop: () => invoke<void>("crawl_stop"),
  cacheClear: (keepIndex = true) => invoke<IndexStats>("cache_clear", { keepIndex }),

  search: (query: string, filter: SearchFilter = {}, limit = 60, offset = 0) =>
    invoke<SearchPage>("search", { query, filter, limit, offset }),

  browse: (path: string, force = false) => invoke<Listing>("browse", { path, force }),

  favorites: () => invoke<Favorite[]>("favorites_list"),
  favoriteAdd: (payload: {
    targetKind: "item" | "folder";
    path: string;
    label?: string;
    note?: string;
    size?: number;
  }) => invoke<void>("favorite_add", payload),
  favoriteRemove: (targetKind: "item" | "folder", path: string) =>
    invoke<void>("favorite_remove", { targetKind, path }),

  lists: () => invoke<ListSummary[]>("lists_all"),
  listCreate: (name: string, kind: "playlist" | "watchlist") =>
    invoke<number>("list_create", { name, kind }),
  listDelete: (listId: number) => invoke<void>("list_delete", { listId }),
  listRename: (listId: number, name: string) => invoke<void>("list_rename", { listId, name }),
  listItems: (listId: number) => invoke<ListItem[]>("list_items", { listId }),
  listAdd: (listId: number, path: string, name?: string) =>
    invoke<void>("list_add", { listId, path, name }),
  listRemove: (listId: number, path: string) => invoke<void>("list_remove", { listId, path }),
  listReorder: (listId: number, paths: string[]) =>
    invoke<void>("list_reorder", { listId, paths }),
  listMarkWatched: (listId: number, path: string, watched: boolean) =>
    invoke<void>("list_mark_watched", { listId, path, watched }),

  counters: () => invoke<Counter[]>("counters_all"),
  counterGet: (path: string) => invoke<Counter>("counter_get", { path }),

  settings: () => invoke<Record<string, string>>("settings_get"),
  settingSet: (key: string, value: string) => invoke<void>("settings_set", { key, value }),
  recentSearches: () => invoke<string[]>("recent_searches"),

  downloadEnqueue: (paths: string[], dest?: string) =>
    invoke<number[]>("download_enqueue", { paths, dest: dest ?? null }),
  downloads: () => invoke<DownloadRow[]>("downloads_list"),
  downloadPause: (id: number) => invoke<void>("download_pause", { id }),
  downloadCancel: (id: number) => invoke<void>("download_cancel", { id }),
  downloadResume: (id: number) => invoke<void>("download_resume", { id }),
  downloadsClear: () => invoke<void>("downloads_clear"),

  exportM3u: (listId: number) => invoke<string>("export_m3u", { listId }),
  play: (path: string) => invoke<void>("play", { path }),
};
