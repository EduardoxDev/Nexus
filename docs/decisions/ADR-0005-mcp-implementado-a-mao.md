# ADR-0005 — Servidor MCP implementado sobre JSON-RPC próprio

- Estado: aceita (implementada na Fase 5)
- Data: 2026-10-10; revisada em 2026-10-12

## Contexto

O NEXUS precisa de um subconjunto pequeno do MCP: transporte stdio, negociação de versão, `tools/list` e `tools/call`. O SDK oficial em Rust evolui rápido e traz dependências assíncronas amplas.

Na verificação do início da Fase 5 (2026-10-12), a revisão vigente era a **2026-07-28**, que tornou o protocolo sem estado: removeu o handshake `initialize`, passou versão e capacidades para o `_meta` de cada requisição e criou `server/discover`. Clientes em uso ainda falam as revisões 2025-xx, com handshake.

## Decisão

1. Implementar o subconjunto diretamente sobre JSON-RPC 2.0, em `crates/nexus-mcp` (biblioteca testável sem processo) e `nexus mcp serve` (adaptador stdio).
2. Servidor **dual-era**, como prevê a especificação: requisições com `_meta["io.modelcontextprotocol/protocolVersion"] = "2026-07-28"` são atendidas sem estado; `initialize` seleciona a semântica legada (2025-11-25, 2025-06-18 ou 2025-03-26) para o processo. Versões desconhecidas recebem `-32022` com a lista suportada.
3. Ferramentas com `inputSchema` fechado (`additionalProperties: false`), `outputSchema` e `structuredContent`, mais o mesmo JSON em texto para compatibilidade. Erros de uso viram `isError: true`; erros de protocolo, JSON-RPC.
4. Sem resources, prompts, subscriptions, tasks, sampling, elicitation ou HTTP enquanto as ferramentas forem suficientes.
5. Conformidade testada por sessão (in-process e com o binário real) e por validação de cada saída contra o `outputSchema` declarado.

## Consequências

- Sem dependências novas (nem assíncronas); o servidor é síncrono e processa uma mensagem por vez — adequado a um cliente por processo stdio.
- Acompanhar a especificação é trabalho manual: uma nova revisão exige revisar `protocol.rs`/`server.rs` e os testes de sessão.

## Revisão

Reavaliar se a superfície necessária crescer (resources, prompts, transporte HTTP) ou se clientes legados deixarem de existir.
