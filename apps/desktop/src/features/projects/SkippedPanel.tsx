import { CircleCheck } from "lucide-react";

import { EmptyState, ErrorState } from "@/components/feedback";
import { Panel } from "@/components/layout";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { t } from "@/lib/i18n/pt-BR";
import { useSkippedFiles } from "@/lib/queries";

const tone = (status: string) =>
  status === "error" ? "danger" : status === "skipped_sensitive" ? "warning" : "neutral";

export function SkippedPanel({ projectId }: { projectId: number }) {
  const skipped = useSkippedFiles(projectId);
  const [fileCol, reasonCol, kindCol] = t.projects.skippedColumns;
  return (
    <Panel title={t.projects.skippedTitle} description={t.projects.skippedDescription}>
      {skipped.isError ? (
        <ErrorState error={skipped.error} onRetry={() => void skipped.refetch()} />
      ) : !skipped.data ? (
        <Skeleton className="h-16" />
      ) : skipped.data.length === 0 ? (
        <EmptyState icon={CircleCheck} title={t.projects.skippedEmpty} />
      ) : (
        <div className="max-h-80 overflow-y-auto">
          <table className="w-full table-fixed border-collapse text-sm">
            <thead className="sticky top-0 bg-surface-1">
              <tr className="text-left text-xs text-fg-subtle">
                <th scope="col" className="w-[42%] pb-2 font-medium">
                  {fileCol}
                </th>
                <th scope="col" className="pb-2 font-medium">
                  {reasonCol}
                </th>
                <th scope="col" className="w-32 pb-2 font-medium">
                  {kindCol}
                </th>
              </tr>
            </thead>
            <tbody>
              {skipped.data.map((f) => (
                <tr key={f.relPath} className="border-t border-border">
                  <td
                    className="selectable truncate py-1.5 pr-2 font-mono text-xs"
                    title={f.relPath}
                  >
                    {f.relPath}
                  </td>
                  <td className="truncate py-1.5 pr-2 text-fg-muted" title={f.reason ?? ""}>
                    {f.reason ?? "—"}
                  </td>
                  <td className="py-1.5">
                    <Badge tone={tone(f.status)}>{t.fileStatus[f.status] ?? f.status}</Badge>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </Panel>
  );
}
