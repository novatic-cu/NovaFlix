import type { ReactNode } from "react";
import { ChevronRight, File, Folder } from "lucide-react";
import { decodePath, formatBytes } from "../lib/format";

interface ItemRowProps {
  kind: "file" | "dir";
  title: string;
  meta?: ReactNode;
  selected?: boolean;
  onClick?: () => void;
  onDoubleClick?: () => void;
  actions?: ReactNode;
}

export function ItemRow({
  kind,
  title,
  meta,
  selected,
  onClick,
  onDoubleClick,
  actions,
}: ItemRowProps) {
  const Icon = kind === "dir" ? Folder : File;

  return (
    <div
      className="row"
      data-selected={selected || undefined}
      onClick={onClick}
      onDoubleClick={onDoubleClick}
      role="button"
      tabIndex={0}
      onKeyDown={(event) => {
        if (event.key === "Enter") onDoubleClick?.();
      }}
    >
      <Icon
        size={16}
        strokeWidth={1.7}
        style={{ color: kind === "dir" ? "var(--color-accent)" : "var(--color-muted)" }}
      />
      <div className="row-main">
        <div className="row-title">{decodePath(title)}</div>
        {meta && <div className="row-meta">{meta}</div>}
      </div>
      <div className="row-actions">{actions}</div>
    </div>
  );
}

export function MetaDot() {
  return <span style={{ opacity: 0.45 }}>·</span>;
}

export function sizeLabel(size?: number | null) {
  return formatBytes(size);
}

export function Chevron() {
  return <ChevronRight size={14} style={{ color: "var(--color-muted)" }} />;
}
