import { useEffect, useState } from "react";
import { Plus } from "lucide-react";
import { api } from "../lib/ipc";
import type { ListSummary } from "../lib/types";
import { useUi } from "../store";
import { Sheet } from "./Sheet";

interface AddToListProps {
  open: boolean;
  path: string;
  name: string;
  onClose: () => void;
}

export function AddToList({ open, path, name, onClose }: AddToListProps) {
  const toast = useUi((state) => state.toast);
  const [lists, setLists] = useState<ListSummary[]>([]);
  const [newName, setNewName] = useState("");
  const [newKind, setNewKind] = useState<"playlist" | "watchlist">("watchlist");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    api
      .lists()
      .then(setLists)
      .catch((error) => toast(String(error), "error"));
  }, [open, toast]);

  const add = async (listId: number) => {
    setBusy(true);
    try {
      await api.listAdd(listId, path, name);
      toast("Añadido a la lista", "success");
      onClose();
    } catch (error) {
      toast(String(error), "error");
    } finally {
      setBusy(false);
    }
  };

  const createAndAdd = async () => {
    if (!newName.trim()) return;
    setBusy(true);
    try {
      const id = await api.listCreate(newName.trim(), newKind);
      await api.listAdd(id, path, name);
      toast(`Lista "${newName.trim()}" creada`, "success");
      setNewName("");
      onClose();
    } catch (error) {
      toast(String(error), "error");
    } finally {
      setBusy(false);
    }
  };

  return (
    <Sheet open={open} title="Añadir a lista" subtitle={name} onClose={onClose}>
      <div className="panel-list" style={{ borderTop: 0 }}>
        {lists.length === 0 && (
          <div className="caption" style={{ padding: "8px 0" }}>
            Aún no hay listas. Crea la primera abajo.
          </div>
        )}
        {lists.map((list) => (
          <button
            key={list.id}
            className="panel-item"
            disabled={busy}
            onClick={() => add(list.id)}
          >
            <span>{list.name}</span>
            <span className="caption">
              {list.kind === "watchlist" ? "Para ver" : "Playlist"} · {list.count}
            </span>
          </button>
        ))}
      </div>

      <div style={{ marginTop: 18, paddingTop: 14, borderTop: "1px solid var(--color-line)" }}>
        <div className="field">
          <label>Nueva lista</label>
          <div style={{ display: "flex", gap: 8 }}>
            <input
              className="input"
              style={{ flex: 1 }}
              value={newName}
              placeholder="Nombre de la lista"
              onChange={(event) => setNewName(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") createAndAdd();
              }}
            />
            <select
              className="select"
              value={newKind}
              onChange={(event) => setNewKind(event.target.value as "playlist" | "watchlist")}
            >
              <option value="watchlist">Para ver</option>
              <option value="playlist">Playlist</option>
            </select>
            <button className="btn" onClick={createAndAdd} disabled={busy || !newName.trim()}>
              <Plus size={14} /> Crear
            </button>
          </div>
        </div>
      </div>
    </Sheet>
  );
}
