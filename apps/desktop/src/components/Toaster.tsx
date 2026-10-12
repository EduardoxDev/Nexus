import { CircleAlert, CircleCheck, Info, X } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/cn";
import { t } from "@/lib/i18n/pt-BR";
import { useToasts, type Toast } from "@/stores/toast";
import { useUi } from "@/stores/ui";

const ICONS = { success: CircleCheck, danger: CircleAlert, info: Info } as const;
const TONES = {
  success: "text-success",
  danger: "text-danger",
  info: "text-accent-text",
} as const;

/** Pilha de notificações no canto inferior direito, acima da barra de status. */
export function Toaster() {
  const toasts = useToasts((s) => s.toasts);
  return (
    <section
      aria-label={t.toast.region}
      className="pointer-events-none fixed right-4 bottom-10 z-50 flex w-[360px] flex-col gap-2"
    >
      <AnimatePresence initial={false}>
        {toasts.map((toast) => (
          <motion.div
            key={toast.id}
            layout
            initial={{ opacity: 0, x: 24, scale: 0.98 }}
            animate={{ opacity: 1, x: 0, scale: 1 }}
            exit={{ opacity: 0, x: 24, transition: { duration: 0.15 } }}
            transition={{ type: "spring", stiffness: 420, damping: 34 }}
          >
            <ToastCard toast={toast} />
          </motion.div>
        ))}
      </AnimatePresence>
    </section>
  );
}

function ToastCard({ toast }: { toast: Toast }) {
  const dismiss = useToasts((s) => s.dismiss);
  const navigate = useUi((s) => s.navigate);
  const selectProject = useUi((s) => s.selectProject);
  const Icon = ICONS[toast.tone];
  return (
    <div
      role={toast.tone === "danger" ? "alert" : "status"}
      className="pointer-events-auto flex items-start gap-3 rounded-xl border border-border bg-surface-1 px-4 py-3 shadow-raised"
    >
      <Icon className={cn("mt-0.5 size-4 shrink-0", TONES[toast.tone])} aria-hidden="true" />
      <div className="min-w-0 flex-1">
        <p className="text-sm font-medium text-fg">{toast.title}</p>
        {toast.body && <p className="mt-0.5 text-xs text-fg-muted">{toast.body}</p>}
        {toast.action && (
          <Button
            size="sm"
            variant="ghost"
            className="-ml-2.5 mt-1 text-accent-text"
            onClick={() => {
              if (toast.action?.projectId !== undefined) selectProject(toast.action.projectId);
              if (toast.action) navigate(toast.action.route);
              dismiss(toast.id);
            }}
          >
            {toast.action.label}
          </Button>
        )}
      </div>
      <button
        type="button"
        onClick={() => dismiss(toast.id)}
        aria-label={t.toast.close}
        className="-mr-1 rounded-md p-1 text-fg-subtle transition-colors hover:bg-surface-2 hover:text-fg"
      >
        <X className="size-3.5" aria-hidden="true" />
      </button>
    </div>
  );
}
