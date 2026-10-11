# Convenções de código

- Exportações nomeadas; nada de `export default`.
- Valores monetários sempre em centavos (`number` inteiro) — use `src/utils/money.ts`.
- Nunca leia `process.env` fora de `src/config/index.ts`.
- Toda rota nova precisa de teste em `tests/` espelhando a pasta de `src/`.
- Erros de domínio estendem `DomainError` e têm um `code` estável.
- Logs passam por `src/utils/logger.ts`; não use `console.log`.
