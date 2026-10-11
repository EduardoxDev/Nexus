import { Pool } from "pg";

import { loadConfig } from "../config";

let pool: Pool | null = null;

/** Pool de conexões com o Postgres (lazy). */
export function db(): Pool {
  if (!pool) {
    pool = new Pool({ connectionString: loadConfig().databaseUrl, max: 10 });
  }
  return pool;
}
