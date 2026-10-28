import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "./ipc";
import { absoluteUrl, isVideo } from "./format";
import { useUi } from "../store";

export function useItemActions() {
  const toast = useUi((state) => state.toast);
  const openFolder = useUi((state) => state.openFolder);
  const baseUrl = useUi((state) => state.baseUrl);

  const download = async (paths: string | string[]) => {
    try {
      const list = Array.isArray(paths) ? paths : [paths];
      await api.downloadEnqueue(list);
      toast(
        list.length === 1 ? "Descarga añadida a la cola" : `${list.length} descargas en cola`,
        "success",
      );
    } catch (error) {
      toast(`No se pudo descargar: ${String(error)}`, "error");
    }
  };

  const play = async (path: string) => {
    try {
      await api.play(path);
      return;
    } catch {
      try {
        await openUrl(absoluteUrl(baseUrl, path));
        toast("Reproductor no encontrado: abriendo en el navegador", "info");
      } catch (error) {
        toast(`No se pudo abrir: ${String(error)}`, "error");
      }
    }
  };

  const downloadAndPlay = async (path: string, ext?: string | null) => {
    if (isVideo(ext)) {
      await play(path);
    } else {
      await download(path);
    }
  };

  const favorite = async (
    targetKind: "item" | "folder",
    path: string,
    label?: string,
    size?: number | null,
  ) => {
    try {
      await api.favoriteAdd({
        targetKind,
        path,
        label,
        size: size ?? undefined,
      });
      toast("Añadido a favoritos", "success");
    } catch (error) {
      toast(`No se pudo guardar: ${String(error)}`, "error");
    }
  };

  const removeFavorite = async (targetKind: "item" | "folder", path: string) => {
    try {
      await api.favoriteRemove(targetKind, path);
      toast("Favorito eliminado", "success");
    } catch (error) {
      toast(String(error), "error");
    }
  };

  const copyUrl = async (path: string) => {
    try {
      await navigator.clipboard.writeText(absoluteUrl(baseUrl, path));
      toast("URL copiada", "success");
    } catch {
      toast("No se pudo copiar la URL", "error");
    }
  };

  const openInBrowser = async (path: string) => {
    try {
      await openUrl(absoluteUrl(baseUrl, path));
    } catch (error) {
      toast(String(error), "error");
    }
  };

  return {
    download,
    play,
    downloadAndPlay,
    favorite,
    removeFavorite,
    copyUrl,
    openInBrowser,
    openFolder,
  };
}
