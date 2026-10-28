import { useCallback, useEffect, useState } from "react";
import { Copy, Download, FolderOpen, Play, Star, Trash2 } from "lucide-react";
import { api } from "../lib/ipc";
import { useItemActions } from "../lib/actions";
import { decodePath, formatDate, isVideo } from "../lib/format";
import type { Favorite } from "../lib/types";
import { useUi } from "../store";
import { ItemRow } from "../components/ItemRow";

export function FavoritesScreen() {
  const toast = useUi((state) => state.toast);
  const openFolder = useUi((state) => state.openFolder);
  const actions = useItemActions();
  const [tab, setTab] = useState<"item" | "folder">("item");
  const [favorites, setFavorites] = useState<Favorite[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(() => {
    setLoading(true);
    api
      .favorites()
      .then(setFavorites)
      .catch((error) => toast(String(error), "error"))
      .finally(() => setLoading(false));
  }, [toast]);

  useEffect(load, [load]);

  const visible = favorites.filter((favorite) => favorite.target_kind === tab);

  const remove = async (favorite: Favorite) => {
    await actions.removeFavorite(favorite.target_kind, favorite.path);
    load();
  };

  return (
    <>
      <header className="topbar">
        <h1 className="title">Favoritos</h1>
        <span className="caption" style={{ marginLeft: "auto" }}>
          {favorites.length} guardados
        </span>
      </header>

      <div className="screen">
        <div className="tabs">
          <button className="tab" data-active={tab === "item"} onClick={() => setTab("item")}>
            Contenidos
          </button>
          <button
            className="tab"
            data-active={tab === "folder"}
            onClick={() => setTab("folder")}
          >
            Carpetas
          </button>
        </div>

        {!loading && visible.length === 0 && (
          <div className="empty">
            <Star className="empty-icon" strokeWidth={1.4} />
            <div className="title">
              {tab === "item" ? "Sin contenidos guardados" : "Sin carpetas guardadas"}
            </div>
            <div className="caption">
              Usa el icono de estrella en cualquier resultado o carpeta.
            </div>
          </div>
        )}

        <div style={{ marginTop: 8 }}>
          {visible.map((favorite) => {
            const kind = favorite.target_kind === "folder" ? "dir" : "file";
            const video = isVideo(favorite.path.split(".").pop() ?? "");
            return (
              <ItemRow
                key={`${favorite.target_kind}:${favorite.path}`}
                kind={kind}
                title={favorite.label || decodePath(favorite.path)}
                meta={
                  <>
                    <span className="badge" data-tone="accent">
                      {favorite.target_kind === "folder" ? "carpeta" : "contenido"}
                    </span>
                    <span>{formatDate(favorite.created_at)}</span>
                    <span style={{ overflow: "hidden", textOverflow: "ellipsis" }}>
                      {decodePath(favorite.path)}
                    </span>
                  </>
                }
                onDoubleClick={() => {
                  if (kind === "dir") openFolder(favorite.path);
                  else void actions.play(favorite.path);
                }}
                actions={
                  <>
                    {kind === "dir" ? (
                      <button
                        className="icon-btn"
                        title="Abrir carpeta"
                        onClick={() => openFolder(favorite.path)}
                      >
                        <FolderOpen size={14} />
                      </button>
                    ) : (
                      <button
                        className="icon-btn"
                        title={video ? "Reproducir" : "Descargar"}
                        onClick={() => void actions.downloadAndPlay(favorite.path, video ? "mkv" : null)}
                      >
                        {video ? <Play size={14} /> : <Download size={14} />}
                      </button>
                    )}
                    {kind === "file" && (
                      <button
                        className="icon-btn"
                        title="Descargar"
                        onClick={() => void actions.download(favorite.path)}
                      >
                        <Download size={14} />
                      </button>
                    )}
                    <button
                      className="icon-btn"
                      title="Copiar URL"
                      onClick={() => void actions.copyUrl(favorite.path)}
                    >
                      <Copy size={14} />
                    </button>
                    <button
                      className="icon-btn"
                      data-tone="danger"
                      title="Quitar"
                      onClick={() => void remove(favorite)}
                    >
                      <Trash2 size={14} />
                    </button>
                  </>
                }
              />
            );
          })}
        </div>
      </div>
    </>
  );
}
