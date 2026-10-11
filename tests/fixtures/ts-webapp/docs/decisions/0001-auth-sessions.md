# ADR 0001 — Sessões assinadas em vez de JWT no navegador

Estado: aceita

## Decisão

A autenticação usa sessões armazenadas no servidor, identificadas por um cookie `HttpOnly`.
Tokens JWT não são guardados no `localStorage`.

## Consequências

- `createSession` e `validateSession` em `src/auth/session.ts` são a única porta de entrada.
- Login social deve reutilizar `createSession` depois de validar o provedor.
