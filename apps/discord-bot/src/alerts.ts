/**
 * Avisos internos para o canal da equipe: alertas de atualidade novos (documento citando arquivo
 * que sumiu, decisão superada, contradições) e indexação que falhou, por projeto da equipe.
 * Cada alerta é avisado uma vez; se for resolvido e voltar, é avisado de novo.
 */
import type { Tools } from "./commands/nexus.ts";
import type { BotState } from "./state.ts";

export interface StaleIssue {
  id: number;
  kind: string;
  severity: string;
  title: string;
  reason: string;
  recommendation: string;
  sources: { relPath: string; line: number | null }[];
}

interface StaleResult {
  issues: StaleIssue[];
}

interface IndexStatus {
  projects: {
    name: string;
    rootExists: boolean;
    lastJob: { id: string; status: string; finishedAt: number | null } | null;
  }[];
}

export interface Notice {
  project: string;
  issues: StaleIssue[];
  failedJob: { id: string; finishedAt: number | null } | null;
  rootMissing: boolean;
}

const INITIALIZED = "alerts.initialized";

/**
 * Novidades desde a última passada. Na primeira execução só registra o que já existe (não
 * despeja alertas antigos no canal).
 */
export async function collectNotices(
  tools: Tools,
  projects: string[],
  state: BotState,
): Promise<Notice[]> {
  const firstRun = state.get(INITIALIZED) === null;
  const out: Notice[] = [];
  for (const project of projects) {
    const stale = await tools.callTool<StaleResult>("get_stale_context", { project });
    const ids = stale.issues.map((i) => String(i.id));
    const scope = `alert:${project}`;
    const fresh = new Set(state.unseen(scope, ids));
    state.forgetExcept(scope, ids);

    const status = await tools.callTool<IndexStatus>("get_index_status", { project });
    const p = status.projects[0];
    const job = p?.lastJob?.status === "failed" ? p.lastJob : null;
    const failedJob =
      job && state.unseen(`job:${project}`, [job.id]).length > 0
        ? { id: job.id, finishedAt: job.finishedAt }
        : null;
    const rootScope = `root:${project}`;
    let rootMissing = false;
    if (p && !p.rootExists) rootMissing = state.unseen(rootScope, ["missing"]).length > 0;
    else state.forgetExcept(rootScope, []);

    const notice: Notice = {
      project,
      issues: stale.issues.filter((i) => fresh.has(String(i.id))),
      failedJob,
      rootMissing,
    };
    if (notice.issues.length > 0 || notice.failedJob || notice.rootMissing) out.push(notice);
  }
  if (firstRun) {
    state.set(INITIALIZED, String(Date.now()));
    return [];
  }
  return out;
}
