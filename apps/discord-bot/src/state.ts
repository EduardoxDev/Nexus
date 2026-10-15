/**
 * Estado do próprio bot (não o banco da NEXUS), num SQLite local: o que já foi anunciado, para
 * não repetir depois de reiniciar. Usa o `node:sqlite` embutido no Node 24.
 */
import { DatabaseSync } from "node:sqlite";

/** Ação de moderação registrada (do automod ou da equipe). */
export interface ModAction {
  userId: string;
  /** Quem agiu: ID do moderador, ou `automod`. */
  by: string;
  action: "warn" | "timeout" | "ban";
  reason: string;
  /** Regra do automod que disparou, se foi ele. */
  rule: string | null;
  /** Duração do silêncio, em minutos. */
  minutes: number | null;
  at: number;
}

export class BotState {
  private readonly db: DatabaseSync;

  /** `path` = arquivo do banco, ou `:memory:` nos testes. */
  constructor(path: string) {
    this.db = new DatabaseSync(path);
    this.db.exec(`
      PRAGMA journal_mode = WAL;
      CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value TEXT NOT NULL) STRICT;
      CREATE TABLE IF NOT EXISTS seen (
        scope TEXT NOT NULL,
        id    TEXT NOT NULL,
        at    INTEGER NOT NULL,
        PRIMARY KEY (scope, id)
      ) STRICT;
      CREATE TABLE IF NOT EXISTS mod_actions (
        id      INTEGER PRIMARY KEY,
        user_id TEXT NOT NULL,
        by      TEXT NOT NULL,
        action  TEXT NOT NULL CHECK (action IN ('warn', 'timeout', 'ban')),
        reason  TEXT NOT NULL,
        rule    TEXT,
        minutes INTEGER,
        at      INTEGER NOT NULL
      ) STRICT;
      CREATE INDEX IF NOT EXISTS mod_actions_user ON mod_actions (user_id, at DESC);
    `);
  }

  get(key: string): string | null {
    const row = this.db.prepare("SELECT value FROM kv WHERE key = ?").get(key) as
      { value: string } | undefined;
    return row?.value ?? null;
  }

  set(key: string, value: string): void {
    this.db
      .prepare(
        "INSERT INTO kv (key, value) VALUES (?, ?) ON CONFLICT (key) DO UPDATE SET value = excluded.value",
      )
      .run(key, value);
  }

  /** Dos `ids`, os que ainda não foram vistos neste `scope`; marca todos como vistos. */
  unseen(scope: string, ids: string[]): string[] {
    const has = this.db.prepare("SELECT 1 FROM seen WHERE scope = ? AND id = ?");
    const add = this.db.prepare("INSERT OR IGNORE INTO seen (scope, id, at) VALUES (?, ?, ?)");
    const fresh = ids.filter((id) => has.get(scope, id) === undefined);
    const now = Date.now();
    for (const id of ids) add.run(scope, id, now);
    return fresh;
  }

  /** Esquece o que não está mais em `keep` (um alerta resolvido que volte é avisado de novo). */
  forgetExcept(scope: string, keep: string[]): void {
    const rows = this.db.prepare("SELECT id FROM seen WHERE scope = ?").all(scope) as {
      id: string;
    }[];
    const del = this.db.prepare("DELETE FROM seen WHERE scope = ? AND id = ?");
    for (const { id } of rows) if (!keep.includes(id)) del.run(scope, id);
  }

  /** Registra uma ação de moderação (quem, o quê, quando, motivo). */
  recordAction(a: ModAction): void {
    this.db
      .prepare(
        "INSERT INTO mod_actions (user_id, by, action, reason, rule, minutes, at) VALUES (?, ?, ?, ?, ?, ?, ?)",
      )
      .run(a.userId, a.by, a.action, a.reason, a.rule, a.minutes, a.at);
  }

  /** Ações sobre a pessoa, da mais recente para a mais antiga. */
  history(userId: string, limit = 10): ModAction[] {
    const rows = this.db
      .prepare(
        "SELECT user_id, by, action, reason, rule, minutes, at FROM mod_actions WHERE user_id = ? ORDER BY at DESC, id DESC LIMIT ?",
      )
      .all(userId, limit) as {
      user_id: string;
      by: string;
      action: ModAction["action"];
      reason: string;
      rule: string | null;
      minutes: number | null;
      at: number;
    }[];
    return rows.map((r) => ({
      userId: r.user_id,
      by: r.by,
      action: r.action,
      reason: r.reason,
      rule: r.rule,
      minutes: r.minutes,
      at: r.at,
    }));
  }

  /** Advertências recebidas desde `since` (ms). */
  warningsSince(userId: string, since: number): number {
    const row = this.db
      .prepare(
        "SELECT COUNT(*) AS n FROM mod_actions WHERE user_id = ? AND action = 'warn' AND at >= ?",
      )
      .get(userId, since) as { n: number };
    return Number(row.n);
  }

  close(): void {
    this.db.close();
  }
}
