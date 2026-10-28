import { useCallback, useEffect, useMemo, useState } from "react";
import {
  ArrowUp,
  Copy,
  Download,
  ExternalLink,
  FolderOpen,
  Play,
  Plus,
  RefreshCw,
  Star,
} from "lucide-react";
import { api } from "../lib/ipc";
import { useItemActions } from "../lib/actions";
import {
  breadcrumbs,
  decodePath,
  formatBytes,
  formatDate,
  isVideo,
} from "../lib/format";
import type { Listing } from "../lib/types";
import { useUi } from "../store";
import { AddToList } from "../components/AddToList";
import { ItemRow, MetaDot } from "../components/ItemRow";

export function BrowseScreen() {
  const path = useUi((state) => state.browsePath);
  const openFolder = useUi((state) => state.openFolder);
  const toast = useUi((state) => state.toast);
  const actions = useItemActions();

  const [listing, setListing] = useState<Listing | null>(null);
  const [loading, setLoading] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [sheet, setSheet] = useState<{ path: string; name: string } | null>(null);
  const [nonce, setNonce] = useState(0);

  const load = useCallback(
    async (force: boolean) => {
      setLoading(true);
      try {
        const result = await api.browse(path, force);
        setListing(result);
      } catch (error) {
        toast(`No se pudo abrir la carpeta: ${String(error)}`, "error");
        setListing(null);
      } finally {
        setLoading(false);
      }
    },
    [path, toast],
  );

  useEffect(() => {
    void load(false);
  }, [load, nonce]);

  const entries = useMemo(() => {
    const items = listing?.entries ?? [];
    return [...items].sort((left, right) => {
      if (left.kind !== right.kind) return left.kind === "dir" ? -1 : 1;
      return left.name.localeCompare(right.name, "es");
    });
  }, [listing]);

  const videos = entries.filter((entry) => isVideo(entry.ext)).map((entry) => entry.path);
  const crumbs = breadcrumbs(path);

  return (
    <>
      <header className="topbar">
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 4,
            flex: 1,
            minWidth: 0,
            overflow: "hidden",
          }}
        >
          {crumbs.map((crumb, index) => (
            <span key={crumb.path} style={{ display: "flex", alignItems: "center", gap: 4 }}>
              {index > 0 && (
                <span style={{ color: "var(--color-muted)", opacity: 0.6 }}>/</span>
              )}
              <button
                className="btn"
                data-variant="ghost"
                style={{ padding: "4px 6px", whiteSpace: "nowrap" }}
                onClick={() => openFolder(crumb.path)}
              >
                {crumb.label}
              </button>
            </span>
          ))}
        </div>
        {listing?.cached && <span className="badge">caché</span>}
        {videos.length > 1 && (
          <button className="btn" onClick={() => void actions.download(videos)}>
            <Download size={14} /> {videos.length} vídeos
          </button>
        )}
        <button className="btn" onClick={() => setNonce((value) => value + 1)} disabled={loading}>
          <RefreshCw size={14} className={loading ? "spin" : undefined} /> Actualizar
        </button>
      </header>

      <div className="screen">
        <div className="screen-pad" style={{ display: "flex", alignItems: "baseline", gap: 10 }}>
          <h1 className="display">{crumbs[crumbs.length - 1]?.label}</h1>
          <span className="caption">
            {loading ? "cargando…" : `${entries.length} elementos`}
          </span>
          {path !== "/" && (
            <button
              className="btn"
              data-variant="ghost"
              style={{ marginLeft: "auto" }}
              onClick={() => openFolder(crumbs[crumbs.length - 2]?.path ?? "/")}
            >
              <ArrowUp size={14} /> Subir
            </button>
          )}
        </div>

        {!loading && entries.length === 0 && (
          <div className="empty">
            <FolderOpen className="empty-icon" strokeWidth={1.4} />
            <div className="title">Carpeta vacía o inaccesible</div>
            <div className="caption">Prueba a actualizar o vuelve a la raíz.</div>
          </div>
        )}

        <div style={{ marginTop: 12 }}>
          {entries.map((entry) => {
            const video = isVideo(entry.ext);
            return (
              <ItemRow
                key={entry.path}
                kind={entry.kind}
                title={entry.name}
                selected={selected === entry.path}
                onClick={() => setSelected(entry.path)}
                onDoubleClick={() => {
                  if (entry.kind === "dir") openFolder(entry.path);
                  else void actions.downloadAndPlay(entry.path, entry.ext);
                }}
                meta={
                  <>
                    {entry.kind === "file" && (
                      <span className="badge" data-tone={video ? "accent" : undefined}>
                        {entry.ext ?? "archivo"}
                      </span>
                    )}
                    {entry.size ? <span>{formatBytes(entry.size)}</span> : null}
                    {entry.mtime && <span>{formatDate(entry.mtime)}</span>}
                    {entry.kind === "file" && (
                      <>
                        <MetaDot />
                        <span>{decodePath(entry.path)}</span>
                      </>
                    )}
                  </>
                }
                actions={
                  <>
                    {entry.kind === "dir" ? (
                      <button
                        className="icon-btn"
                        title="Abrir"
                        onClick={(event) => {
                          event.stopPropagation();
                          openFolder(entry.path);
                        }}
                      >
                        <FolderOpen size={14} />
                      </button>
                    ) : (
                      <button
                        className="icon-btn"
                        title={video ? "Reproducir" : "Descargar"}
                        onClick={(event) => {
                          event.stopPropagation();
                          void actions.downloadAndPlay(entry.path, entry.ext);
                        }}
                      >
                        {video ? <Play size={14} /> : <Download size={14} />}
                      </button>
                    )}
                    {entry.kind === "file" && (
                      <button
                        className="icon-btn"
                        title="Descargar"
                        onClick={(event) => {
                          event.stopPropagation();
                          void actions.download(entry.path);
                        }}
                      >
                        <Download size={14} />
                      </button>
                    )}
                    <button
                      className="icon-btn"
                      title="Favorito"
                      onClick={(event) => {
                        event.stopPropagation();
                        void actions.favorite(
                          entry.kind === "dir" ? "folder" : "item",
                          entry.path,
                          entry.name,
                          entry.size,
                        );
                      }}
                    >
                      <Star size={14} />
                    </button>
                    <button
                      className="icon-btn"
                      title="Añadir a lista"
                      onClick={(event) => {
                        event.stopPropagation();
                        setSheet({ path: entry.path, name: entry.name });
                      }}
                    >
                      <Plus size={14} />
                    </button>
                    <button
                      className="icon-btn"
                      title="Copiar URL"
                      onClick={(event) => {
                        event.stopPropagation();
                        void actions.copyUrl(entry.path);
                      }}
                    >
                      <Copy size={14} />
                    </button>
                    <button
                      className="icon-btn"
                      title="Abrir en navegador"
                      onClick={(event) => {
                        event.stopPropagation();
                        void actions.openInBrowser(entry.path);
                      }}
                    >
                      <ExternalLink size={14} />
                    </button>
                  </>
                }
              />
            );
          })}
        </div>
      </div>

      <AddToList
        open={!!sheet}
        path={sheet?.path ?? ""}
        name={sheet?.name ?? ""}
        onClose={() => setSheet(null)}
      />
    </>
  );
}
