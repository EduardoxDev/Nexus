import { verifyPassword } from "../../auth/password";
import { providers } from "../../auth/oauth/provider";
import { createSession, revokeSession } from "../../auth/session";
import { db } from "../../db/client";
import type { Route } from "../app";

export const authRoutes: Route[] = [
  {
    method: "POST",
    path: "/api/auth/login",
    async handler(req) {
      const { email, password } = (await req.json()) as { email: string; password: string };
      const { rows } = await db().query("SELECT id, password_hash FROM users WHERE email = $1", [email]);
      const user = rows[0];
      if (!user || !(await verifyPassword(user.password_hash, password))) {
        return new Response("invalid credentials", { status: 401 });
      }
      const session = await createSession(user.id);
      return new Response(null, { status: 204, headers: { "set-cookie": `sid=${session.id}; HttpOnly` } });
    },
  },
  {
    method: "POST",
    path: "/api/auth/logout",
    async handler(req) {
      const sid = /sid=([\w-]+)/.exec(req.headers.get("cookie") ?? "")?.[1];
      if (sid) await revokeSession(sid);
      return new Response(null, { status: 204 });
    },
  },
  {
    method: "GET",
    path: "/api/auth/oauth/:provider/callback",
    async handler(req) {
      const url = new URL(req.url);
      const provider = providers.get(url.pathname.split("/")[4] ?? "");
      if (!provider) return new Response("unknown provider", { status: 404 });
      const { email, verified } = await provider.exchangeCode(url.searchParams.get("code") ?? "");
      if (!verified) return new Response("email not verified", { status: 403 });
      const { rows } = await db().query("SELECT id FROM users WHERE email = $1", [email]);
      const session = await createSession(rows[0].id);
      return new Response(null, { status: 302, headers: { location: "/", "set-cookie": `sid=${session.id}; HttpOnly` } });
    },
  },
];
