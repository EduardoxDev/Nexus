const WINDOW_MS = 60_000;
const MAX_REQUESTS = 120;
const hits = new Map<string, { count: number; resetAt: number }>();

/** Limita requisições por IP numa janela fixa de 1 minuto. */
export function rateLimit(handler: (req: Request) => Promise<Response>) {
  return async (req: Request): Promise<Response> => {
    const ip = req.headers.get("x-forwarded-for") ?? "local";
    const now = Date.now();
    const entry = hits.get(ip);
    if (!entry || entry.resetAt < now) {
      hits.set(ip, { count: 1, resetAt: now + WINDOW_MS });
    } else if (++entry.count > MAX_REQUESTS) {
      return new Response("too many requests", { status: 429 });
    }
    return handler(req);
  };
}
