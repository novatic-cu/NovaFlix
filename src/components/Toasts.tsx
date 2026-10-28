import { AnimatePresence, motion } from "motion/react";
import { Check, Info, X, TriangleAlert } from "lucide-react";
import { useUi } from "../store";

const ICON = {
  info: Info,
  success: Check,
  error: TriangleAlert,
};

export function Toasts() {
  const toasts = useUi((state) => state.toasts);
  const dismiss = useUi((state) => state.dismiss);

  return (
    <div className="toasts">
      <AnimatePresence initial={false}>
        {toasts.map((toast) => {
          const Icon = ICON[toast.tone];
          return (
            <motion.div
              key={toast.id}
              className="toast"
              layout
              initial={{ opacity: 0, y: 12, scale: 0.98 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: 6, scale: 0.98 }}
              transition={{ type: "spring", bounce: 0, duration: 0.3 }}
            >
              <Icon
                size={15}
                strokeWidth={2}
                style={{
                  color:
                    toast.tone === "error"
                      ? "var(--color-danger)"
                      : toast.tone === "success"
                        ? "var(--color-success)"
                        : "var(--color-accent)",
                  flex: "none",
                }}
              />
              <span style={{ flex: 1 }}>{toast.message}</span>
              <button className="icon-btn" onClick={() => dismiss(toast.id)} aria-label="Cerrar">
                <X size={13} />
              </button>
            </motion.div>
          );
        })}
      </AnimatePresence>
    </div>
  );
}
