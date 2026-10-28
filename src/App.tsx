import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { Sidebar } from "./components/Sidebar";
import { Toasts } from "./components/Toasts";
import { SearchScreen } from "./screens/Search";
import { BrowseScreen } from "./screens/Browse";
import { FavoritesScreen } from "./screens/Favorites";
import { ListsScreen } from "./screens/Lists";
import { DownloadsScreen } from "./screens/Downloads";
import { SettingsScreen } from "./screens/Settings";
import { api } from "./lib/ipc";
import type { DownloadProgress } from "./lib/types";
import { useUi } from "./store";

export default function App() {
  const screen = useUi((state) => state.screen);
  const setScreen = useUi((state) => state.setScreen);
  const setStats = useUi((state) => state.setStats);
  const setBaseUrl = useUi((state) => state.setBaseUrl);
  const setLive = useUi((state) => state.setLive);
  const toast = useUi((state) => state.toast);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        const info = await api.appInfo();
        if (!cancelled) setBaseUrl(info.base_url);
      } catch {
        /* modo navegador: se mantiene la URL por defecto */
      }

      try {
        let stats = await api.indexStats();
        if (stats.dirs < 1000) {
          toast("Indexando el catálogo por primera vez…", "info");
          stats = await api.indexBootstrap(false);
        }
        if (!cancelled) setStats(stats);
      } catch (error) {
        if (!cancelled) toast(`Índice no disponible: ${String(error)}`, "error");
      }
    })();

    const unlisten = listen<DownloadProgress>("download://progress", (event) => {
      setLive(event.payload);
      if (event.payload.state === "done" && event.payload.dest) {
        toast(`Descarga completada: ${event.payload.dest.split("/").pop()}`, "success");
      }
      if (event.payload.state === "error") {
        toast(`Descarga con error: ${event.payload.error ?? "desconocido"}`, "error");
      }
    });

    const keyHandler = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        setScreen("search");
      }
    };
    window.addEventListener("keydown", keyHandler);

    return () => {
      cancelled = true;
      window.removeEventListener("keydown", keyHandler);
      void unlisten.then((off) => off());
    };
  }, [setBaseUrl, setLive, setScreen, setStats, toast]);

  return (
    <div className="app-shell">
      <Sidebar />
      <main className="content">
        {screen === "search" && <SearchScreen />}
        {screen === "browse" && <BrowseScreen />}
        {screen === "favorites" && <FavoritesScreen />}
        {screen === "lists" && <ListsScreen />}
        {screen === "downloads" && <DownloadsScreen />}
        {screen === "settings" && <SettingsScreen />}
      </main>
      <Toasts />
    </div>
  );
}
