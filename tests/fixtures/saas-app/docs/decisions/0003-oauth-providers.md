# ADR 0003 — Provedores OAuth plugáveis

Estado: aceita

## Decisão
Cada provedor OAuth implementa a interface `OAuthProvider` em `src/auth/oauth/provider.ts`
(`authorizationUrl` e `exchangeCode`). O GitHub é a primeira implementação.
Novos provedores (Google, Microsoft) seguem o mesmo contrato e são registrados em `providers`.
Depois de obter o e-mail verificado, o fluxo chama `createSession` (ver ADR 0001).
