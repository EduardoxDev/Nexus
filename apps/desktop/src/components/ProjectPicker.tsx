import { useEffect } from "react";

import { t } from "@/lib/i18n/pt-BR";
import { useProjects } from "@/lib/queries";
import { useUi } from "@/stores/ui";

/** Seleção do projeto ativo (compartilhada entre Projetos, Busca e Context Packs). */
export function ProjectPicker() {
  const projects = useProjects();
  const selected = useUi((s) => s.selectedProjectId);
  const selectProject = useUi((s) => s.selectProject);

  useEffect(() => {
    const list = projects.data;
    if (!list || list.length === 0) return;
    if (selected === null || !list.some((p) => p.project.id === selected)) {
      selectProject(list[0]?.project.id ?? null);
    }
  }, [projects.data, selected, selectProject]);

  if (!projects.data || projects.data.length === 0) return null;
  return (
    <label className="flex items-center gap-2 text-sm text-fg-muted">
      {t.picker.label}
      <select
        value={selected ?? ""}
        onChange={(e) => selectProject(Number(e.target.value))}
        className="h-8 max-w-56 rounded-md border border-border bg-surface-2 px-2 text-sm text-fg hover:border-border-strong"
      >
        {projects.data.map((p) => (
          <option key={p.project.id} value={p.project.id}>
            {p.project.name}
          </option>
        ))}
      </select>
    </label>
  );
}
