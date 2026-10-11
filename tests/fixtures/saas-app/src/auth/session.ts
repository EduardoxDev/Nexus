import { randomUUID } from "node:crypto";

import { loadConfig } from "../config";
import { db } from "../db/client";

export interface Session {
  id: string;
  userId: string;
  expiresAt: Date;
}

/** Cria a sessão do usuário autenticado (ADR 0001). */
export async function createSession(userId: string): Promise<Session> {
  const ttlMs = loadConfig().sessionTtlHours * 3_600_000;
  const session = { id: randomUUID(), userId, expiresAt: new Date(Date.now() + ttlMs) };
  await db().query("INSERT INTO sessions (id, user_id, expires_at) VALUES ($1, $2, $3)", [
    session.id,
    session.userId,
    session.expiresAt,
  ]);
  return session;
}

/** Valida o cookie de sessão; sessões expiradas são removidas. */
export async function validateSession(sessionId: string): Promise<Session | null> {
  const { rows } = await db().query("SELECT id, user_id, expires_at FROM sessions WHERE id = $1", [sessionId]);
  const row = rows[0];
  if (!row) return null;
  if (row.expires_at <= new Date()) {
    await revokeSession(sessionId);
    return null;
  }
  return { id: row.id, userId: row.user_id, expiresAt: row.expires_at };
}

export async function revokeSession(sessionId: string): Promise<void> {
  await db().query("DELETE FROM sessions WHERE id = $1", [sessionId]);
}
