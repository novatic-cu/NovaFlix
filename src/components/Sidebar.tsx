import {
  Download,
  FolderTree,
  ListVideo,
  Search,
  Settings,
  Star,
} from "lucide-react";
import { formatBytes } from "../lib/format";
import { useUi } from "../store";
import type { Screen } from "../lib/types";

const NAV: { id: Screen; label: string; icon: typeof Search }[] = [
  { id: "search", label: "Buscar", icon: Search },
  { id: "browse", label: "Explorar", icon: FolderTree },
  { id: "favorites", label: "Favoritos", icon: Star },
  { id: "lists", label: "Listas", icon: ListVideo },
  { id: "downloads", label: "Descargas", icon: Download },
  { id: "settings", label: "Ajustes", icon: Settings },
];

export function Sidebar() {
  const screen = useUi((state) => state.screen);
  const setScreen = useUi((state) => state.setScreen);
  const stats = useUi((state) => state.stats);

  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-mark">N</div>
        <div>
          <div className="brand-name">NovaFlix</div>
          <div className="brand-sub">Buscador multimedia</div>
        </div>
      </div>

      {NAV.slice(0, 1).map((item) => (
        <button
          key={item.id}
          className="nav-item"
          data-active={screen === item.id}
          onClick={() => setScreen(item.id)}
        >
          <item.icon className="nav-icon" strokeWidth={1.8} />
          {item.label}
        </button>
      ))}

      <div className="nav-label">Biblioteca</div>
      {NAV.slice(1, 5).map((item) => (
        <button
          key={item.id}
          className="nav-item"
          data-active={screen === item.id}
          onClick={() => setScreen(item.id)}
        >
          <item.icon className="nav-icon" strokeWidth={1.8} />
          {item.label}
        </button>
      ))}

      <div className="nav-label">Sistema</div>
      {NAV.slice(5).map((item) => (
        <button
          key={item.id}
          className="nav-item"
          data-active={screen === item.id}
          onClick={() => setScreen(item.id)}
        >
          <item.icon className="nav-icon" strokeWidth={1.8} />
          {item.label}
        </button>
      ))}

      <div className="sidebar-foot">
        <span>{stats ? `${stats.dirs.toLocaleString("es-ES")} carpetas` : "Índice sin cargar"}</span>
        <span>{stats ? `${stats.files.toLocaleString("es-ES")} archivos` : "—"}</span>
        <span>Caché {stats ? formatBytes(stats.cache_bytes) : "—"}</span>
      </div>
    </aside>
  );
}
