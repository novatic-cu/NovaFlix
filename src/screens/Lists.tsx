import { useCallback, useEffect, useState } from "react";
import {
  ArrowDown,
  ArrowUp,
  Check,
  FileDown,
  ListPlus,
  Play,
  Plus,
  Trash2,
} from "lucide-react";
import { openPath } from "@tauri-apps/plugin-opener";
import { api } from "../lib/ipc";
import { useItemActions } from "../lib/actions";
import { decodePath, formatDate } from "../lib/format";
import type { ListItem, ListSummary } from "../lib/types";
import { useUi } from "../store";
import { Sheet } from "../components/Sheet";
import { ItemRow } from "../components/ItemRow";

export function ListsScreen() {
  const toast = useUi((state) => state.toast);
  const actions = useItemActions();
  const [lists, setLists] = useState<ListSummary[]>([]);
  const [activeId, setActiveId] = useState<number | null>(null);
  const [items, setItems] = useState<ListItem[]>([]);
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [kind, setKind] = useState<"playlist" | "watchlist">("playlist");

  const loadLists = useCallback(async () => {
    try {
      const result = await api.lists();
      setLists(result);
      setActiveId((current) => current ?? result[0]?.id ?? null);
    } catch (error) {
      toast(String(error), "error");
    }
  }, [toast]);

  const loadItems = useCallback(
    async (listId: number) => {
      try {
        setItems(await api.listItems(listId));
      } catch (error) {
        toast(String(error), "error");
      }
    },
    [toast],
  );

  useEffect(() => {
    void loadLists();
  }, [loadLists]);

  useEffect(() => {
    if (activeId !== null) void loadItems(activeId);
  }, [activeId, loadItems]);

  const active = lists.find((list) => list.id === activeId) ?? null;

  const create = async () => {
    if (!name.trim()) return;
    try {
      const id = await api.listCreate(name.trim(), kind);
      setName("");
      setCreating(false);
      await loadLists();
      setActiveId(id);
      toast("Lista creada", "success");
    } catch (error) {
      toast(String(error), "error");
    }
  };

  const removeList = async () => {
    if (!active) return;
    try {
      await api.listDelete(active.id);
      setActiveId(null);
      setItems([]);
      await loadLists();
      toast("Lista eliminada", "success");
    } catch (error) {
      toast(String(error), "error");
    }
  };

  const move = async (index: number, direction: -1 | 1) => {
    if (!active) return;
    const target = index + direction;
    if (target < 0 || target >= items.length) return;
    const next = [...items];
    [next[index], next[target]] = [next[target], next[index]];
    setItems(next);
    try {
      await api.listReorder(
        active.id,
        next.map((item) => item.path),
      );
    } catch (error) {
      toast(String(error), "error");
    }
  };

  const toggleWatched = async (item: ListItem) => {
    if (!active) return;
    const watched = !item.watched_at;
    setItems((current) =>
      current.map((entry) =>
        entry.path === item.path
          ? { ...entry, watched_at: watched ? new Date().toISOString() : null }
          : entry,
      ),
    );
    try {
      await api.listMarkWatched(active.id, item.path, watched);
    } catch (error) {
      toast(String(error), "error");
    }
  };

  const removeItem = async (item: ListItem) => {
    if (!active) return;
    try {
      await api.listRemove(active.id, item.path);
      await loadItems(active.id);
      await loadLists();
    } catch (error) {
      toast(String(error), "error");
    }
  };

  const exportM3u = async () => {
    if (!active) return;
    try {
      const file = await api.exportM3u(active.id);
      toast(`M3U generado: ${file}`, "success");
      await openPath(file);
    } catch (error) {
      toast(String(error), "error");
    }
  };

  return (
    <>
      <header className="topbar">
        <h1 className="title">Listas</h1>
        <div style={{ marginLeft: "auto", display: "flex", gap: 8 }}>
          {active && items.length > 0 && (
            <button className="btn" onClick={exportM3u}>
              <FileDown size={14} /> Exportar M3U
            </button>
          )}
          <button className="btn" data-variant="primary" onClick={() => setCreating(true)}>
            <Plus size={14} /> Nueva lista
          </button>
        </div>
      </header>

      <div className="screen">
        {lists.length === 0 ? (
          <div className="empty">
            <ListPlus className="empty-icon" strokeWidth={1.4} />
            <div className="title">Sin listas todavía</div>
            <div className="caption">
              Crea una playlist o una lista de "para ver" y añade contenidos desde la búsqueda.
            </div>
          </div>
        ) : (
          <div className="split">
            <div>
              <div className="caption" style={{ marginBottom: 8 }}>
                {lists.length} listas
              </div>
              <div className="panel-list">
                {lists.map((list) => (
                  <button
                    key={list.id}
                    className="panel-item"
                    data-active={list.id === activeId}
                    onClick={() => setActiveId(list.id)}
                  >
                    <span style={{ overflow: "hidden", textOverflow: "ellipsis" }}>
                      {list.name}
                    </span>
                    <span className="caption">{list.count}</span>
                  </button>
                ))}
              </div>
            </div>

            <div>
              {active ? (
                <>
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 10,
                      paddingBottom: 10,
                      borderBottom: "1px solid var(--color-line)",
                    }}
                  >
                    <div className="title">{active.name}</div>
                    <span className="badge" data-tone="accent">
                      {active.kind === "watchlist" ? "para ver" : "playlist"}
                    </span>
                    <span className="caption">
                      {items.length} elementos · {formatDate(active.updated_at)}
                    </span>
                    <button
                      className="btn"
                      data-variant="danger"
                      style={{ marginLeft: "auto" }}
                      onClick={removeList}
                    >
                      <Trash2 size={14} /> Eliminar
                    </button>
                  </div>

                  {items.length === 0 && (
                    <div className="empty">
                      <ListPlus className="empty-icon" strokeWidth={1.4} />
                      <div className="title">Lista vacía</div>
                      <div className="caption">
                        Añade contenidos con el icono + desde Buscar o Explorar.
                      </div>
                    </div>
                  )}

                  {items.map((item, index) => (
                    <ItemRow
                      key={item.path}
                      kind="file"
                      title={item.name || decodePath(item.path)}
                      selected={!!item.watched_at}
                      meta={
                        <>
                          <span className="badge" data-tone={item.watched_at ? "success" : undefined}>
                            {item.watched_at ? "visto" : "pendiente"}
                          </span>
                          <span>{decodePath(item.path)}</span>
                        </>
                      }
                      onDoubleClick={() => void actions.play(item.path)}
                      actions={
                        <>
                          <button
                            className="icon-btn"
                            title="Subir"
                            onClick={() => void move(index, -1)}
                          >
                            <ArrowUp size={14} />
                          </button>
                          <button
                            className="icon-btn"
                            title="Bajar"
                            onClick={() => void move(index, 1)}
                          >
                            <ArrowDown size={14} />
                          </button>
                          <button
                            className="icon-btn"
                            data-tone={item.watched_at ? "accent" : undefined}
                            title="Marcar visto"
                            onClick={() => void toggleWatched(item)}
                          >
                            <Check size={14} />
                          </button>
                          <button
                            className="icon-btn"
                            title="Reproducir"
                            onClick={() => void actions.play(item.path)}
                          >
                            <Play size={14} />
                          </button>
                          <button
                            className="icon-btn"
                            data-tone="danger"
                            title="Quitar"
                            onClick={() => void removeItem(item)}
                          >
                            <Trash2 size={14} />
                          </button>
                        </>
                      }
                    />
                  ))}
                </>
              ) : (
                <div className="empty">
                  <div className="caption">Selecciona una lista.</div>
                </div>
              )}
            </div>
          </div>
        )}
      </div>

      <Sheet
        open={creating}
        title="Nueva lista"
        subtitle="Playlist ordenada o cola de pendientes"
        onClose={() => setCreating(false)}
      >
        <div className="field">
          <label>Nombre</label>
          <input
            className="input"
            value={name}
            autoFocus
            placeholder="Ej. Ciencia ficción, Pendientes 2026"
            onChange={(event) => setName(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") void create();
            }}
          />
        </div>
        <div className="field">
          <label>Tipo</label>
          <div className="segmented">
            <button data-active={kind === "playlist"} onClick={() => setKind("playlist")}>
              Playlist
            </button>
            <button data-active={kind === "watchlist"} onClick={() => setKind("watchlist")}>
              Para ver
            </button>
          </div>
        </div>
        <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 8 }}>
          <button className="btn" onClick={() => setCreating(false)}>
            Cancelar
          </button>
          <button className="btn" data-variant="primary" onClick={() => void create()}>
            Crear
          </button>
        </div>
      </Sheet>
    </>
  );
}
