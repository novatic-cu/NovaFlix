import { useEffect, useMemo, useRef, useState } from "react";
import {
  ChevronRight,
  Copy,
  Download,
  ExternalLink,
  FolderOpen,
  Play,
  Plus,
  Search as SearchIcon,
  Star,
} from "lucide-react";
import { api } from "../lib/ipc";
import { useItemActions } from "../lib/actions";
import { decodePath, formatBytes, formatDate, isVideo } from "../lib/format";
import type { SearchFilter, SearchPage } from "../lib/types";
import { useUi } from "../store";
import { AddToList } from "../components/AddToList";
import { ItemRow, MetaDot } from "../components/ItemRow";

const EMPTY: SearchPage = { hits: [], total: 0, took_ms: 0 };

export function SearchScreen() {
  const query = useUi((state) => state.searchQuery);
  const setQuery = useUi((state) => state.setSearchQuery);
  const openFolder = useUi((state) => state.openFolder);
  const toast = useUi((state) => state.toast);
  const actions = useItemActions();

  const [kind, setKind] = useState<"all" | "file" | "dir">("all");
  const [ext, setExt] = useState<"" | "video" | "subs">("");
  const [year, setYear] = useState("");
  const [page, setPage] = useState<SearchPage>(EMPTY);
  const [loading, setLoading] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [sheet, setSheet] = useState<{ path: string; name: string } | null>(null);
  const [recent, setRecent] = useState<string[]>([]);
  const inputRef = useRef<HTMLInputElement>(null);
  const requestId = useRef(0);

  const filter = useMemo<SearchFilter>(() => {
    const next: SearchFilter = {};
    if (kind !== "all") next.kind = kind;
    if (ext === "video") next.ext = "mkv";
    if (ext === "subs") next.ext = "srt";
    const parsedYear = Number.parseInt(year, 10);
    if (!Number.isNaN(parsedYear) && parsedYear > 1900) next.year = parsedYear;
    return next;
  }, [kind, ext, year]);

  useEffect(() => {
    inputRef.current?.focus();
    api.recentSearches().then(setRecent).catch(() => undefined);
  }, []);

  useEffect(() => {
    const id = ++requestId.current;
    if (!query.trim()) {
      setPage(EMPTY);
      setLoading(false);
      return;
    }
    setLoading(true);
    const handle = window.setTimeout(() => {
      api
        .search(query, filter, 80)
        .then((result) => {
          if (requestId.current === id) setPage(result);
        })
        .catch((error) => toast(String(error), "error"))
        .finally(() => {
          if (requestId.current === id) setLoading(false);
        });
    }, 140);
    return () => window.clearTimeout(handle);
  }, [query, filter, toast]);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        inputRef.current?.focus();
        inputRef.current?.select();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  return (
    <>
      <header className="topbar">
        <div className="search-shell">
          <SearchIcon size={15} style={{ color: "var(--color-muted)" }} />
          <input
            ref={inputRef}
            className="search-input"
            placeholder="Buscar películas, series, carpetas…"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            spellCheck={false}
            autoComplete="off"
          />
          <span className="kbd">Ctrl K</span>
        </div>
        <button className="btn" onClick={() => setQuery("")} disabled={!query}>
          Limpiar
        </button>
      </header>

      <div className="screen">
        <div className="filters">
          <div className="segmented">
            {(
              [
                ["all", "Todo"],
                ["file", "Archivos"],
                ["dir", "Carpetas"],
              ] as const
            ).map(([value, label]) => (
              <button
                key={value}
                data-active={kind === value}
                onClick={() => setKind(value)}
              >
                {label}
              </button>
            ))}
          </div>

          <div className="segmented">
            {(
              [
                ["", "Cualquiera"],
                ["video", "Vídeo"],
                ["subs", "Subtítulos"],
              ] as const
            ).map(([value, label]) => (
              <button key={value} data-active={ext === value} onClick={() => setExt(value)}>
                {label}
              </button>
            ))}
          </div>

          <input
            className="input"
            style={{ width: 92 }}
            placeholder="Año"
            value={year}
            inputMode="numeric"
            onChange={(event) => setYear(event.target.value.replace(/\D/g, "").slice(0, 4))}
          />
        </div>

        {!query.trim() && (
          <div className="screen-pad">
            <div className="caption" style={{ marginBottom: 10 }}>
              Búsquedas recientes
            </div>
            {recent.length === 0 ? (
              <div className="empty">
                <SearchIcon className="empty-icon" strokeWidth={1.4} />
                <div className="title">Busca en el catálogo</div>
                <div className="caption">
                  Índice local de carpetas y archivos. Prueba "wicked", "dinosaur" o "2025".
                </div>
              </div>
            ) : (
              <div className="panel-list">
                {recent.map((item) => (
                  <button key={item} className="panel-item" onClick={() => setQuery(item)}>
                    <span>{item}</span>
                    <ChevronRight size={14} style={{ color: "var(--color-muted)" }} />
                  </button>
                ))}
              </div>
            )}
          </div>
        )}

        {query.trim() && (
          <>
            <div
              className="caption"
              style={{ display: "flex", gap: 10, padding: "10px 2px 6px" }}
            >
              <span>
                {loading ? "Buscando…" : `${page.total.toLocaleString("es-ES")} resultados`}
              </span>
              {!loading && <span>·</span>}
              {!loading && <span>{page.took_ms} ms</span>}
            </div>

            {page.hits.length === 0 && !loading && (
              <div className="empty">
                <SearchIcon className="empty-icon" strokeWidth={1.4} />
                <div className="title">Sin coincidencias</div>
                <div className="caption">
                  Si buscas algo reciente, actualiza el índice en Ajustes.
                </div>
              </div>
            )}

            <div>
              {page.hits.map((hit) => {
                const video = isVideo(hit.ext);
                return (
                  <ItemRow
                    key={`${hit.kind}:${hit.path}`}
                    kind={hit.kind}
                    title={hit.name || decodePath(hit.path)}
                    selected={selected === hit.path}
                    onClick={() => setSelected(hit.path)}
                    onDoubleClick={() => {
                      if (hit.kind === "dir") openFolder(hit.path);
                      else if (video) void actions.play(hit.path);
                      else void actions.download(hit.path);
                    }}
                    meta={
                      <>
                        <span className="badge" data-tone={hit.kind === "dir" ? "accent" : undefined}>
                          {hit.kind === "dir" ? "Carpeta" : (hit.ext ?? "archivo")}
                        </span>
                        {hit.year && <span>{hit.year}</span>}
                        {hit.size ? <span>{formatBytes(hit.size)}</span> : null}
                        {hit.mtime && <span>{formatDate(hit.mtime)}</span>}
                        <MetaDot />
                        <span
                          style={{
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                            maxWidth: 420,
                          }}
                        >
                          {decodePath(hit.parent)}
                        </span>
                      </>
                    }
                    actions={
                      <>
                        {hit.kind === "dir" ? (
                          <button
                            className="icon-btn"
                            title="Abrir carpeta"
                            onClick={(event) => {
                              event.stopPropagation();
                              openFolder(hit.path);
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
                              void actions.downloadAndPlay(hit.path, hit.ext);
                            }}
                          >
                            {video ? <Play size={14} /> : <Download size={14} />}
                          </button>
                        )}
                        <button
                          className="icon-btn"
                          title="Descargar"
                          onClick={(event) => {
                            event.stopPropagation();
                            void actions.download(hit.path);
                          }}
                        >
                          <Download size={14} />
                        </button>
                        <button
                          className="icon-btn"
                          title="Favorito"
                          onClick={(event) => {
                            event.stopPropagation();
                            void actions.favorite(
                              hit.kind === "dir" ? "folder" : "item",
                              hit.path,
                              hit.name,
                              hit.size,
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
                            setSheet({ path: hit.path, name: hit.name });
                          }}
                        >
                          <Plus size={14} />
                        </button>
                        <button
                          className="icon-btn"
                          title="Copiar URL"
                          onClick={(event) => {
                            event.stopPropagation();
                            void actions.copyUrl(hit.path);
                          }}
                        >
                          <Copy size={14} />
                        </button>
                        <button
                          className="icon-btn"
                          title="Abrir en navegador"
                          onClick={(event) => {
                            event.stopPropagation();
                            void actions.openInBrowser(hit.path);
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
          </>
        )}
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
