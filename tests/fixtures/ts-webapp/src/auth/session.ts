import { randomUUID } from "node:crypto";

export interface Session {
  id: string;
  userId: string;
  expiresAt: number;
}

const SESSION_TTL_MS = 1000 * 60 * 60 * 8;
const sessions = new Map<string, Session>();

/** Cria uma sessão para o usuário autenticado. */
export function createSession(userId: string, now = Date.now()): Session {
  const session = { id: randomUUID(), userId, expiresAt: now + SESSION_TTL_MS };
  sessions.set(session.id, session);
  return session;
}

/** Valida o cookie de sessão; retorna `null` se ausente ou expirado. */
export function validateSession(sessionId: string, now = Date.now()): Session | null {
  const session = sessions.get(sessionId);
  if (!session || session.expiresAt <= now) {
    sessions.delete(sessionId);
    return null;
  }
  return session;
}

export function revokeSession(sessionId: string): void {
  sessions.delete(sessionId);
}
