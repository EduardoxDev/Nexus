import { useState, type SubmitEvent } from "react";

import { ErrorState } from "@/components/feedback";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import type { Memory, MemoryKind, MemoryRefInput } from "@/lib/bindings";
import { t } from "@/lib/i18n/pt-BR";
import { NexusError } from "@/lib/ipc";
import { useMemoryMutations } from "@/lib/queries";

export const MEMORY_KINDS: MemoryKind[] = [
  "decision",
  "convention",
  "requirement",
  "constraint",
  "bug",
  "solution",
  "dependency",
  "task",
  "lesson",
];

/** `caminho`, `caminho:12` ou `caminho:10-20` (mesmo formato da CLI). */
export function parseRefs(text: string): { refs: MemoryRefInput[]; invalid: string | null } {
  const refs: MemoryRefInput[] = [];
  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (!line) continue;
    const match = /^(.+?):(\d+)(?:-(\d+))?$/.exec(line);
    if (match) {
      const [, relPath = "", a = "", b] = match;
      const start = Number(a);
      const end = b === undefined ? start : Number(b);
      if (start < 1 || end < start) return { refs, invalid: line };
      refs.push({ relPath, startLine: start, endLine: end });
    } else {
      refs.push({ relPath: line });
    }
  }
  return { refs, invalid: null };
}

export function formatRefs(memory: Memory): string {
  return memory.refs
    .map((r) =>
      r.startLine !== null && r.endLine !== null
        ? `${r.relPath}:${r.startLine}-${r.endLine}`
        : r.relPath,
    )
    .join("\n");
}

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  projectId: number;
  /** Ausente = criar. */
  memory?: Memory;
  onSaved?: (memory: Memory) => void;
}

export function MemoryEditor({ open, onOpenChange, projectId, memory, onSaved }: Props) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="w-[min(680px,calc(100vw-32px))] p-5">
        {open && (
          <EditorForm
            projectId={projectId}
            onClose={() => onOpenChange(false)}
            {...(memory ? { memory } : {})}
            {...(onSaved ? { onSaved } : {})}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}

function EditorForm({
  projectId,
  memory,
  onSaved,
  onClose,
}: {
  projectId: number;
  memory?: Memory;
  onSaved?: (memory: Memory) => void;
  onClose: () => void;
}) {
  const s = t.memory.editor;
  const [kind, setKind] = useState<MemoryKind>(memory?.kind ?? "decision");
  const [title, setTitle] = useState(memory?.title ?? "");
  const [body, setBody] = useState(memory?.body ?? "");
  const [refsText, setRefsText] = useState(memory ? formatRefs(memory) : "");
  const [note, setNote] = useState("");
  const [confirmNow, setConfirmNow] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);
  const { create, update } = useMemoryMutations();
  const pending = create.isPending || update.isPending;
  const error = memory ? update.error : create.error;

  const onSubmit = (e: SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    const { refs, invalid } = parseRefs(refsText);
    if (invalid !== null) {
      setLocalError(s.invalidRef(invalid));
      return;
    }
    setLocalError(null);
    const done = (m: Memory) => {
      onSaved?.(m);
      onClose();
    };
    const trimmedNote = note.trim();
    if (memory) {
      update.mutate(
        {
          memoryId: memory.id,
          request: {
            expectedRevision: memory.revision,
            kind,
            title,
            body,
            refs,
            ...(trimmedNote ? { note: trimmedNote } : {}),
          },
        },
        { onSuccess: done },
      );
    } else {
      create.mutate(
        {
          projectId,
          kind,
          title,
          body,
          refs,
          confirm: confirmNow,
          ...(trimmedNote ? { note: trimmedNote } : {}),
        },
        { onSuccess: done },
      );
    }
  };

  const field =
    "rounded-lg border border-border bg-surface-1 shadow-card px-2.5 text-sm text-fg placeholder:text-fg-subtle";
  return (
    <form onSubmit={onSubmit} className="flex flex-col gap-3">
      <DialogTitle className="pr-6 text-lg font-semibold">
        {memory ? s.editTitle : s.createTitle}
      </DialogTitle>
      <DialogDescription className="sr-only">{t.memory.emptyBody}</DialogDescription>
      <div className="grid grid-cols-[160px_1fr] gap-3">
        <label className="flex flex-col gap-1 text-xs text-fg-muted">
          {s.kind}
          <select
            value={kind}
            onChange={(e) => setKind(e.target.value as MemoryKind)}
            className={`h-8 ${field}`}
          >
            {MEMORY_KINDS.map((k) => (
              <option key={k} value={k}>
                {t.memoryKinds[k]}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1 text-xs text-fg-muted">
          {s.title}
          <input
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            maxLength={200}
            required
            minLength={3}
            className={`h-8 ${field}`}
          />
        </label>
      </div>
      <label className="flex flex-col gap-1 text-xs text-fg-muted">
        {s.body}
        <textarea
          value={body}
          onChange={(e) => setBody(e.target.value)}
          placeholder={s.bodyPlaceholder}
          maxLength={20000}
          rows={7}
          className={`resize-y py-1.5 ${field}`}
        />
      </label>
      <label className="flex flex-col gap-1 text-xs text-fg-muted">
        {s.refs}
        <textarea
          value={refsText}
          onChange={(e) => setRefsText(e.target.value)}
          placeholder={s.refsPlaceholder}
          rows={3}
          className={`resize-y py-1.5 font-mono text-xs ${field}`}
        />
      </label>
      <label className="flex flex-col gap-1 text-xs text-fg-muted">
        {s.note}
        <input
          value={note}
          onChange={(e) => setNote(e.target.value)}
          maxLength={1000}
          className={`h-8 ${field}`}
        />
      </label>
      {!memory && (
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={confirmNow}
            onChange={(e) => setConfirmNow(e.target.checked)}
            className="size-4 accent-[var(--accent)]"
          />
          {s.confirmNow}
        </label>
      )}
      {localError && (
        <p role="alert" className="text-sm text-danger">
          {localError}
        </p>
      )}
      {error &&
        (error instanceof NexusError && error.code === "conflict" && memory ? (
          <p role="alert" className="text-sm text-danger">
            {s.conflict}
          </p>
        ) : (
          <ErrorState error={error} />
        ))}
      <div className="mt-2 flex justify-end gap-2">
        <Button type="button" onClick={onClose} disabled={pending}>
          {s.cancel}
        </Button>
        <Button type="submit" variant="primary" disabled={pending || title.trim().length < 3}>
          {pending ? s.saving : s.save}
        </Button>
      </div>
    </form>
  );
}
