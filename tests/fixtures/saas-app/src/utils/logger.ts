type Level = "debug" | "info" | "warn" | "error";

/** Logger estruturado em JSON. Nunca registre senhas ou tokens. */
export function log(level: Level, message: string, fields: Record<string, unknown> = {}): void {
  process.stdout.write(JSON.stringify({ level, message, ...fields, at: new Date().toISOString() }) + "\n");
}
