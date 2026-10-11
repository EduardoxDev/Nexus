/** Contrato de um provedor OAuth (ADR 0003). */
export interface OAuthProvider {
  readonly name: string;
  authorizationUrl(state: string): string;
  /** Troca o código de autorização pelo e-mail verificado do usuário. */
  exchangeCode(code: string): Promise<{ email: string; verified: boolean }>;
}

export const providers = new Map<string, OAuthProvider>();

export function registerProvider(provider: OAuthProvider): void {
  providers.set(provider.name, provider);
}
