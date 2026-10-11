# Arquitetura

A aplicação tem três camadas:

1. **HTTP** (`src/server`): rotas finas que validam entrada e chamam os serviços.
2. **Domínio** (`src/auth`, `src/billing`, `src/checkout`, `src/orders`, `src/notifications`).
3. **Infraestrutura** (`src/db`, `src/config`, `src/utils`).

O frontend (`src/web`) conversa apenas com a API HTTP.
Pagamentos passam pelo cliente Stripe em `src/billing/stripeClient.ts`.
