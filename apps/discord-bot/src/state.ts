/**
 * Estado do próprio bot (não o banco da NEXUS), num SQLite local: o que já foi anunciado, para
 * não repetir depois de reiniciar. Usa o `node:sqlite` embutido no Node 24.
 */
import { DatabaseSync } from "node:sqlite";

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

  close(): void {
    this.db.close();
  }
}
