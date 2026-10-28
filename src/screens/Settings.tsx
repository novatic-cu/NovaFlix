import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Boxes,
  Database,
  FolderSearch,
  HardDrive,
  RefreshCw,
  Square,
} from "lucide-react";
import { api } from "../lib/ipc";
import { formatBytes, formatDate } from "../lib/format";
import type { AppInfo, CrawlProgress } from "../lib/types";
import { useUi } from "../store";

const DEFAULTS: Record<string, string> = {
  index_mode: "light",
  cache_ttl_hours: "12",
  bootstrap_ttl_hours: "24",
  cache_max_mb: "64",
  player: "auto",
  download_dir: "",
};

export function SettingsScreen() {
  const toast = useUi((state) => state.toast);
  const stats = useUi((state) => state.stats);
  const setStats = useUi((state) => state.setStats);
  const [values, setValues] = useState<Record<string, string>>(DEFAULTS);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [crawl, setCrawl] = useState<CrawlProgress | null>(null);

  const loadStats = useCallback(async () => {
    try {
      setStats(await api.indexStats());
    } catch (error) {
      toast(String(error), "error");
    }
  }, [setStats, toast]);

  useEffect(() => {
    api.settings().then((loaded) => setValues({ ...DEFAULTS, ...loaded })).catch(() => undefined);
    api.appInfo().then(setInfo).catch(() => undefined);
    void loadStats();
  }, [loadStats]);

  useEffect(() => {
    const unlisten = listen<CrawlProgress>("crawl://progress", (event) => {
      setCrawl(event.payload);
    });
    return () => {
      void unlisten.then((off) => off());
    };
  }, []);

  const save = async (key: string, value: string) => {
    setValues((current) => ({ ...current, [key]: value }));
    try {
      await api.settingSet(key, value);
    } catch (error) {
      toast(String(error), "error");
    }
  };

  const runBootstrap = async () => {
    setBusy("bootstrap");
    try {
      const result = await api.indexBootstrap(true);
      setStats(result);
      toast(`Índice actualizado: ${result.dirs.toLocaleString("es-ES")} carpetas`, "success");
    } catch (error) {
      toast(String(error), "error");
    } finally {
      setBusy(null);
    }
  };

  const startCrawl = async () => {
    setBusy("crawl");
    setCrawl(null);
    try {
      await api.crawlStart();
      toast("Crawl profundo iniciado en segundo plano", "info");
    } catch (error) {
      toast(String(error), "error");
    } finally {
      setBusy(null);
    }
  };

  const stopCrawl = async () => {
    try {
      await api.crawlStop();
      toast("Crawl detenido", "info");
      await loadStats();
    } catch (error) {
      toast(String(error), "error");
    }
  };

  const clearCache = async (keepIndex: boolean) => {
    try {
      const result = await api.cacheClear(keepIndex);
      setStats(result);
      toast(keepIndex ? "Caché de carpetas vaciada" : "Índice completo eliminado", "success");
    } catch (error) {
      toast(String(error), "error");
    }
  };

  const pickFolder = async () => {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (typeof selected === "string") {
        await save("download_dir", selected);
      }
    } catch (error) {
      toast(String(error), "error");
    }
  };

  return (
    <>
      <header className="topbar">
        <h1 className="title">Ajustes</h1>
        <span className="caption" style={{ marginLeft: "auto" }}>
          {info ? `v${info.version}` : ""}
        </span>
      </header>

      <div className="screen">
        <div className="screen-pad">
          <div className="caption" style={{ marginBottom: 4 }}>
            Índice
          </div>

          <div className="settings-grid">
            <div>
              <div style={{ fontWeight: 550 }}>Modo de índice</div>
              <div className="hint">
                Ligero usa el árbol público del sitio y lista carpetas bajo demanda. Profundo
                recorre todo el catálogo (más lento, más caché).
              </div>
            </div>
            <div className="segmented">
              <button
                data-active={values.index_mode !== "deep"}
                onClick={() => save("index_mode", "light")}
              >
                Ligero
              </button>
              <button
                data-active={values.index_mode === "deep"}
                onClick={() => save("index_mode", "deep")}
              >
                Profundo
              </button>
            </div>
          </div>

          <div className="settings-grid">
            <div>
              <div style={{ fontWeight: 550 }}>Actualizar árbol publicado</div>
              <div className="hint">
                Descarga listado.html y refresca el índice de carpetas.
              </div>
            </div>
            <div style={{ display: "flex", gap: 8 }}>
              <button className="btn" data-variant="primary" onClick={runBootstrap} disabled={!!busy}>
                <RefreshCw size={14} /> Actualizar
              </button>
              <button className="btn" onClick={startCrawl}>
                <Database size={14} /> Crawl profundo
              </button>
              {busy === "crawl" && (
                <button className="btn" data-variant="danger" onClick={stopCrawl}>
                  <Square size={13} /> Detener
                </button>
              )}
            </div>
          </div>

          {crawl && (
            <div className="meter">
              <div style={{ display: "flex", justifyContent: "space-between" }}>
                <span className="caption">Crawl en curso</span>
                <span className="caption">
                  {crawl.dirs.toLocaleString("es-ES")} carpetas · {crawl.files.toLocaleString("es-ES")} archivos
                </span>
              </div>
              <div className="progress-track">
                <div
                  className="progress-fill"
                  style={{ width: `${Math.min(100, (crawl.dirs / Math.max(crawl.max, 1)) * 100)}%` }}
                />
              </div>
              <div className="caption">
                En cola {crawl.queued.toLocaleString("es-ES")} · errores {crawl.errors}
              </div>
            </div>
          )}

          <div className="settings-grid">
            <div>
              <div style={{ fontWeight: 550 }}>Caché de listados</div>
              <div className="hint">Tope en disco para los directorios visitados (LRU).</div>
            </div>
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              <input
                className="input"
                style={{ width: 90 }}
                type="number"
                min={8}
                max={2048}
                value={values.cache_max_mb}
                onChange={(event) => setValues((c) => ({ ...c, cache_max_mb: event.target.value }))}
                onBlur={(event) => save("cache_max_mb", event.target.value)}
              />
              <span className="caption">MB</span>
              <button className="btn" style={{ marginLeft: "auto" }} onClick={() => clearCache(true)}>
                Vaciar caché
              </button>
            </div>
          </div>

          <div className="settings-grid">
            <div>
              <div style={{ fontWeight: 550 }}>Vigencia de la caché</div>
              <div className="hint">Horas antes de volver a pedir un listado al servidor.</div>
            </div>
            <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
              <input
                className="input"
                style={{ width: 90 }}
                type="number"
                min={1}
                max={720}
                value={values.cache_ttl_hours}
                onChange={(event) =>
                  setValues((c) => ({ ...c, cache_ttl_hours: event.target.value }))
                }
                onBlur={(event) => save("cache_ttl_hours", event.target.value)}
              />
              <span className="caption">horas</span>
            </div>
          </div>

          <div className="settings-grid">
            <div>
              <div style={{ fontWeight: 550 }}>Carpeta de descargas</div>
              <div className="hint">{values.download_dir || info?.downloads_dir || "—"}</div>
            </div>
            <div style={{ display: "flex", gap: 8 }}>
              <button className="btn" onClick={pickFolder}>
                <FolderSearch size={14} /> Elegir carpeta
              </button>
              <button className="btn" data-variant="ghost" onClick={() => save("download_dir", "")}>
                Usar predeterminada
              </button>
            </div>
          </div>

          <div className="settings-grid">
            <div>
              <div style={{ fontWeight: 550 }}>Reproductor externo</div>
              <div className="hint">
                Comando a ejecutar (vlc, mpv, celluloid…). "auto" prueba en orden.
              </div>
            </div>
            <input
              className="input"
              value={values.player}
              onChange={(event) => setValues((c) => ({ ...c, player: event.target.value }))}
              onBlur={(event) => save("player", event.target.value)}
            />
          </div>

          <div className="caption" style={{ margin: "26px 0 4px" }}>
            Estado
          </div>

          <div className="settings-grid">
            <div className="hint" style={{ display: "grid", gap: 4 }}>
              <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
                <Boxes size={13} /> Carpetas indexadas: {stats?.dirs.toLocaleString("es-ES") ?? "—"}
              </span>
              <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
                <HardDrive size={13} /> Archivos indexados: {stats?.files.toLocaleString("es-ES") ?? "—"}
              </span>
              <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
                <Database size={13} /> Caché: {formatBytes(stats?.cache_bytes)} en{" "}
                {stats?.cached_dirs ?? 0} carpetas
              </span>
            </div>
            <div className="hint" style={{ display: "grid", gap: 4 }}>
              <span>Árbol: {stats?.last_bootstrap ? formatDate(stats.last_bootstrap) : "nunca"}</span>
              <span>Crawl: {stats?.last_crawl ? formatDate(stats.last_crawl) : "nunca"}</span>
              <span>Datos: {info?.data_dir ?? "—"}</span>
            </div>
          </div>

          <div className="settings-grid" style={{ borderBottom: 0 }}>
            <div>
              <div style={{ fontWeight: 550 }}>Restablecer índice</div>
              <div className="hint">Elimina carpetas y archivos indexados. No toca favoritos ni listas.</div>
            </div>
            <div style={{ justifySelf: "start" }}>
              <button className="btn" data-variant="danger" onClick={() => clearCache(false)}>
                Borrar índice
              </button>
            </div>
          </div>

          <p className="hint" style={{ marginTop: 24, maxWidth: 620 }}>
            NovaFlix es un cliente de escritorio: no aloja contenido. Solo indexa el listado
            público de {info?.base_url ?? "visuales.uclv.cu"} y descarga bajo tu decisión.
          </p>
        </div>
      </div>
    </>
  );
}
