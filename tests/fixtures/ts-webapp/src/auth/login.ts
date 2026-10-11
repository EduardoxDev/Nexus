import { z } from "zod";

import { createSession, type Session } from "./session";

const credentialsSchema = z.object({
  email: z.string().email(),
  password: z.string().min(8),
});

export interface UserRepository {
  findByEmail(email: string): Promise<{ id: string; passwordHash: string } | null>;
  verifyPassword(hash: string, password: string): Promise<boolean>;
}

/** Fluxo de login por e-mail e senha. */
export async function loginWithPassword(
  repo: UserRepository,
  input: unknown,
): Promise<Session> {
  const credentials = credentialsSchema.parse(input);
  const user = await repo.findByEmail(credentials.email);
  if (!user || !(await repo.verifyPassword(user.passwordHash, credentials.password))) {
    throw new Error("invalid_credentials");
  }
  return createSession(user.id);
}
