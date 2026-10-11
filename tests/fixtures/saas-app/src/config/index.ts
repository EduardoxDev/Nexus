/** Único ponto que lê variáveis de ambiente. */
export interface AppConfig {
  databaseUrl: string;
  stripeSecretKey: string;
  smtpHost: string;
  sessionTtlHours: number;
}

function required(name: string): string {
  const value = process.env[name];
  if (!value) throw new Error(`missing env var ${name}`);
  return value;
}

export function loadConfig(): AppConfig {
  return {
    databaseUrl: required("DATABASE_URL"),
    stripeSecretKey: required("STRIPE_SECRET_KEY"),
    smtpHost: process.env.SMTP_HOST ?? "localhost",
    sessionTtlHours: Number(process.env.SESSION_TTL_HOURS ?? "8"),
  };
}
