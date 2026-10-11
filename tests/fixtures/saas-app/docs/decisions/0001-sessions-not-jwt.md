# ADR 0001 — Sessões no servidor em vez de JWT

Estado: aceita

## Contexto
Precisamos revogar acessos imediatamente (logout, troca de senha).

## Decisão
Usar sessões armazenadas no servidor com cookie `HttpOnly`. Tokens JWT não são emitidos para o navegador.
Toda autenticação nova (incluindo login social) termina chamando `createSession`.

## Consequências
`requireSession` é o único middleware que libera rotas autenticadas.
