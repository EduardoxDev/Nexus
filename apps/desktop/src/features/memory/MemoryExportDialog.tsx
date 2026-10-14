import { ErrorState } from "@/components/feedback";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Skeleton } from "@/components/ui/skeleton";
import type { ExportAction } from "@/lib/bindings";
import { t } from "@/lib/i18n/pt-BR";
import { useMemoryExport, useMemoryExportPreview } from "@/lib/queries";
import { useToasts } from "@/stores/toast";

const s = t.memory.share;

const TONE: Record<ExportAction, "accent" | "neutral" | "success"> = {
  create: "success",
  update: "accent",
  unchanged: "neutral",
};

/**
 * "Exportar para o repositório": mostra o que será criado ou alterado em `.nexus/memory/` e só
 * grava com o clique de confirmação. A gravação é recusada se algo mudou desde a prévia.
 */
export function MemoryExportDialog({
  projectId,
  open,
  onOpenChange,
}: {
  projectId: number;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const preview = useMemoryExportPreview(open ? projectId : null);
  const exporter = useMemoryExport();
  const push = useToasts((st) => st.push);
  const data = preview.data;
  // Fechar ou abrir de novo começa sem o erro da tentativa anterior.
  const changeOpen = (next: boolean) => {
    exporter.reset();
    onOpenChange(next);
  };
  const pending = data?.items.filter((i) => i.action !== "unchanged").length ?? 0;

  const confirm = () => {
    if (!data) return;
    exporter.mutate(
      { projectId, token: data.token },
      {
        onSuccess: (report) => {
          push({
            tone: "success",
            title: s.doneTitle,
            body: s.done(report.created, report.updated, report.directory),
          });
          changeOpen(false);
        },
        onError: () => void preview.refetch(),
      },
    );
  };

  return (
    <Dialog open={open} onOpenChange={changeOpen}>
      <DialogContent className="w-[min(640px,calc(100vw-32px))] p-5">
        <DialogTitle className="text-lg font-semibold">{s.title}</DialogTitle>
        <DialogDescription className="mt-1 text-sm text-fg-muted">
          {s.description}
        </DialogDescription>

        <div className="mt-4 max-h-[50vh] overflow-y-auto">
          {preview.isError ? (
            <ErrorState error={preview.error} onRetry={() => void preview.refetch()} />
          ) : !data ? (
            <Skeleton className="h-24" />
          ) : data.items.length === 0 ? (
            <p className="text-sm text-fg-muted">{s.nothing}</p>
          ) : (
            <ul aria-label={s.listLabel} className="flex flex-col divide-y divide-border">
              {data.items.map((item) => (
                <li key={item.memoryId} className="flex items-center gap-3 py-2">
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm text-fg">{item.title}</p>
                    <p className="truncate font-mono text-xs text-fg-subtle">{item.relPath}</p>
                  </div>
                  <Badge tone={TONE[item.action]}>{s.actions[item.action]}</Badge>
                </li>
              ))}
            </ul>
          )}
          {data && data.obsolete.length > 0 && (
            <div className="mt-4 rounded-lg border border-border bg-surface-2 p-3 text-xs">
              <p className="text-fg-muted">{s.obsolete}</p>
              <ul className="mt-1.5 flex flex-col gap-0.5">
                {data.obsolete.map((f) => (
                  <li key={f.relPath} className="font-mono text-fg">
                    {f.relPath}
                  </li>
                ))}
              </ul>
            </div>
          )}
          {exporter.isError && <ErrorState className="mt-3" error={exporter.error} />}
        </div>

        <div className="mt-5 flex items-center justify-end gap-2">
          {data && pending === 0 && data.items.length > 0 && (
            <p className="mr-auto text-xs text-fg-muted">{s.upToDate}</p>
          )}
          <Button variant="ghost" onClick={() => changeOpen(false)}>
            {t.common.close}
          </Button>
          <Button
            variant="primary"
            onClick={confirm}
            // Enquanto a prévia recarrega, o token mostrado pode estar velho.
            disabled={!data || pending === 0 || exporter.isPending || preview.isFetching}
          >
            {exporter.isPending ? s.writing : s.confirm(pending)}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
