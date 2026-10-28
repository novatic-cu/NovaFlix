import { useCallback, useEffect, useMemo, useState } from "react";
import { Download, Pause, Play, RotateCw, Trash2, X } from "lucide-react";
import { api } from "../lib/ipc";
import { decodePath, formatBytes } from "../lib/format";
import type { DownloadRow } from "../lib/types";
import { useUi } from "../store";

const STATE_LABEL: Record<DownloadRow["state"], string> = {
  queued: "en cola",
  active: "descargando",
  paused: "pausada",
  done: "completada",
  error: "error",
  cancelled: "cancelada",
};

const STATE_TONE: Record<DownloadRow["state"], "accent" | "success" | "warn" | "danger" | undefined> =
  {
    queued: undefined,
    active: "accent",
    paused: "warn",
    done: "success",
    error: "danger",
    cancelled: undefined,
  };

export function DownloadsScreen() {
  const toast = useUi((state) => state.toast);
  const live = useUi((state) => state.live);
  const [rows, setRows] = useState<DownloadRow[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(() => {
    setLoading(true);
    api
      .downloads()
      .then(setRows)
      .catch((error) => toast(String(error), "error"))
      .finally(() => setLoading(false));
  }, [toast]);

  useEffect(load, [load]);

  const merged = useMemo(() => {
    return rows.map((row) => {
      const progress = live[row.id];
      if (!progress) return row;
      return {
        ...row,
        state: progress.state,
        bytes_done: progress.done || row.bytes_done,
        bytes_total: progress.total || row.bytes_total,
        error: progress.error ?? row.error,
      };
    });
  }, [rows, live]);

  const activeCount = merged.filter((row) =>
    ["active", "queued", "paused"].includes(row.state),
  ).length;

  const act = async (action: () => Promise<void>, reload = true) => {
    try {
      await action();
      if (reload) load();
    } catch (error) {
      toast(String(error), "error");
    }
  };

  return (
    <>
      <header className="topbar">
        <h1 className="title">Descargas</h1>
        <span className="caption">{activeCount} en curso</span>
        <div style={{ marginLeft: "auto", display: "flex", gap: 8 }}>
          <button className="btn" onClick={load} disabled={loading}>
            <RotateCw size={14} /> Refrescar
          </button>
          <button
            className="btn"
            data-variant="danger"
            onClick={() =>
              act(async () => {
                await api.downloadsClear();
                toast("Historial limpio", "success");
              })
            }
          >
            <Trash2 size={14} /> Limpiar completadas
          </button>
        </div>
      </header>

      <div className="screen">
        {merged.length === 0 && !loading && (
          <div className="empty">
            <Download className="empty-icon" strokeWidth={1.4} />
            <div className="title">Sin descargas</div>
            <div className="caption">
              Descarga desde Buscar o Explorar. Se reanudan con Range si se interrumpen.
            </div>
          </div>
        )}

        {merged.map((row) => {
          const percent =
            row.bytes_total > 0
              ? Math.min(100, Math.round((row.bytes_done / row.bytes_total) * 100))
              : row.state === "done"
                ? 100
                : 0;
          const name = decodePath(row.path.split("/").pop() ?? row.path);
          return (
            <div key={row.id} className="row" style={{ gridTemplateColumns: "22px 1fr auto" }}>
              <Download
                size={16}
                strokeWidth={1.7}
                style={{
                  color:
                    row.state === "done" ? "var(--color-success)" : "var(--color-accent)",
                }}
              />
              <div className="row-main">
                <div className="row-title">{name}</div>
                <div className="row-meta" style={{ marginBottom: 6 }}>
                  <span className="badge" data-tone={STATE_TONE[row.state]}>
                    {STATE_LABEL[row.state]}
                  </span>
                  <span>
                    {formatBytes(row.bytes_done)} / {formatBytes(row.bytes_total)}
                  </span>
                  <span>{percent}%</span>
                  {row.error && <span style={{ color: "var(--color-danger)" }}>{row.error}</span>}
                </div>
                <div className="progress-track">
                  <div className="progress-fill" style={{ width: `${percent}%` }} />
                </div>
                <div className="caption" style={{ marginTop: 4, fontSize: 11 }}>
                  {row.dest}
                </div>
              </div>
              <div className="row-actions" style={{ opacity: 1 }}>
                {row.state === "active" || row.state === "queued" ? (
                  <button
                    className="icon-btn"
                    title="Pausar"
                    onClick={() => act(() => api.downloadPause(row.id), false)}
                  >
                    <Pause size={14} />
                  </button>
                ) : null}
                {["paused", "error", "cancelled"].includes(row.state) ? (
                  <button
                    className="icon-btn"
                    title="Reanudar"
                    onClick={() => act(() => api.downloadResume(row.id))}
                  >
                    <Play size={14} />
                  </button>
                ) : null}
                {["active", "queued", "paused"].includes(row.state) ? (
                  <button
                    className="icon-btn"
                    data-tone="danger"
                    title="Cancelar"
                    onClick={() => act(() => api.downloadCancel(row.id))}
                  >
                    <X size={14} />
                  </button>
                ) : null}
              </div>
            </div>
          );
        })}
      </div>
    </>
  );
}
