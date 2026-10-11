# ADR-0002 — Um banco SQLite compartilhado em WAL

- Estado: aceita
- Data: 2026-10-10

## Contexto

Desktop, CLI e servidor MCP precisam enxergar o mesmo índice, a mesma memória e o mesmo histórico. Um serviço/daemon separado aumentaria a superfície operacional.

## Decisão

- Arquivo único `nexus.db` no diretório de dados (`%LOCALAPPDATA%\Nexus` no Windows; `NEXUS_DATA_DIR` sobrescreve).
- `journal_mode=WAL`, `busy_timeout=5000`, `foreign_keys=ON`, `synchronous=NORMAL`.
- Escritas em transações curtas; leitura de arquivos e hashing acontecem **fora** de transações.
- A partir da Fase 2, indexação de um projeto é protegida por lock registrado no banco (pid + heartbeat), com recuperação de locks órfãos.
- O desktop roda como instância única.

## Consequências

Sem servidor para administrar; concorrência entre processos depende do locking do SQLite e de transações curtas. Bancos em pastas de rede não são suportados (documentado em troubleshooting).
