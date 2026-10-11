import { ErrorState } from "@/components/feedback";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { t } from "@/lib/i18n/pt-BR";
import { useRemoveProject } from "@/lib/queries";
import { useUi } from "@/stores/ui";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  projectId: number;
  name: string;
}

/** Confirmação explícita: a remoção apaga apenas dados do índice, nunca arquivos. */
export function RemoveProjectDialog({ open, onOpenChange, projectId, name }: Props) {
  const remove = useRemoveProject();
  const selectProject = useUi((s) => s.selectProject);
  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!remove.isPending) {
          remove.reset();
          onOpenChange(next);
        }
      }}
    >
      <DialogContent className="p-5">
        <DialogTitle className="pr-6 text-lg font-semibold">
          {t.projects.removeDialog.title(name)}
        </DialogTitle>
        <DialogDescription className="mt-2 text-sm text-fg-muted">
          {t.projects.removeDialog.body}
        </DialogDescription>
        {remove.isError && <ErrorState className="mt-3" error={remove.error} />}
        <div className="mt-5 flex justify-end gap-2">
          <Button onClick={() => onOpenChange(false)} disabled={remove.isPending} autoFocus>
            {t.projects.removeDialog.cancel}
          </Button>
          <Button
            variant="danger"
            disabled={remove.isPending}
            onClick={() =>
              remove.mutate(projectId, {
                onSuccess: () => {
                  selectProject(null);
                  onOpenChange(false);
                },
              })
            }
          >
            {t.projects.removeDialog.confirm}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
