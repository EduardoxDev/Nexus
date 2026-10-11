# ADR-0005 — Servidor MCP implementado sobre JSON-RPC próprio

- Estado: proposta (implementação na Fase 5)
- Data: 2026-10-10

## Contexto

O NEXUS precisa de um subconjunto pequeno do MCP: transporte stdio, `initialize` com negociação de versão, `tools/list`, `tools/call`, `ping` e notificações. O SDK oficial em Rust evolui rápido e traz dependências assíncronas amplas.

## Decisão

Implementar o subconjunto necessário diretamente sobre JSON-RPC 2.0, seguindo a revisão vigente da especificação (verificada no início da Fase 5), com testes de conformidade por sessão e validação com o MCP Inspector e com Claude Code.

## Revisão

Reavaliar se a superfície necessária crescer (resources, prompts, sampling, transporte HTTP).
