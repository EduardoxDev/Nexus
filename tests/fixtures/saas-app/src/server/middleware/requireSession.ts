import { validateSession, type Session } from "../../auth/session";

/** Único middleware que libera rotas autenticadas (ADR 0001). */
export function requireSession(
  handler: (req: Request, session: Session) => Promise<Response>,
): (req: Request) => Promise<Response> {
  return async (req) => {
    const cookie = req.headers.get("cookie") ?? "";
    const sessionId = /sid=([\w-]+)/.exec(cookie)?.[1];
    const session = sessionId ? await validateSession(sessionId) : null;
    if (!session) return new Response("unauthorized", { status: 401 });
    return handler(req, session);
  };
}
