# Roadmap

Cada fase termina com aplicação executável e testes aprovados. Critérios de aceitação são verificáveis; metas de desempenho só são registradas depois de medidas.

| Fase | Tema                         | Estado    |
| ---- | ---------------------------- | --------- |
| 0    | Reconhecimento e arquitetura | concluída |
| 1    | Fundação desktop             | concluída |
| 2    | Indexação de projetos        | planejada |
| 3    | Busca e Context Packs        | planejada |
| 4    | Memória e atualidade         | planejada |
| 5    | MCP e CLI                    | planejada |
| 6    | Inteligência avançada        | planejada |
| 7    | Hardening e release          | planejada |

## Fase 0 — Reconhecimento e arquitetura

Entregas: inspeção do ambiente, matriz de versões verificada ([development.md](development.md)), arquitetura ([architecture.md](architecture.md)), ADRs em [`decisions/`](decisions/), modelo de ameaças inicial, plano de schema, este roadmap.

## Fase 1 — Fundação desktop

Entregas:

- Workspace Cargo + pnpm; `nexus-core` com storage (migrations versionadas com checksum, backup pré-migração, recusa de downgrade), configurações com allowlist, log de operações, diagnóstico e verificação de integridade, logging estruturado.
- CLI `nexus init` e `nexus diagnostics [--json]`.
- App Tauri 2 + React com design system (tema escuro/claro), shell (sidebar, breadcrumbs, barra de status), command palette (Ctrl+K), telas Visão geral, Configurações e Diagnóstico com dados reais.
- Contratos TypeScript gerados a partir do Rust.
- Testes Rust, Vitest e E2E Playwright contra o app real; CI no GitHub Actions; instalador NSIS de desenvolvimento.

Critérios de aceitação (verificados em 2026-10-10):

- [x] Primeira execução cria o banco com schema v1; execuções seguintes não reaplicam migrations (testes `core.rs`, CLI e E2E).
- [x] Diagnóstico mostra apenas dados reais; "Verificar integridade" registra operação com duração (Vitest e E2E).
- [x] Tema e movimento reduzido persistem após fechar e reabrir (E2E com WM_CLOSE + reabertura).
- [x] CLI e desktop operam sobre o mesmo banco (`NEXUS_DATA_DIR` / `%LOCALAPPDATA%\Nexus`).
- [x] Instalador NSIS de desenvolvimento instala, abre (E2E 5/5 no app instalado) e desinstala silenciosamente.
- [x] fmt, clippy (`-D warnings`), testes Rust (44), tsc, eslint, prettier, Vitest (54) e E2E (5) verdes localmente.
- [ ] CI executado no GitHub — pendente: o repositório ainda não tem remoto.

Medições registradas (não são metas): o teste E2E de abertura (iniciar o executável de depuração, conectar via CDP e ver a Visão geral) levou entre 1,4 s e 2,6 s nas execuções locais; instalador de depuração: 3,67 MiB; bundle do frontend: ~560 KiB de JS.

## Fase 2 — Indexação de projetos

- Seleção de pasta, registro de projetos (remover do índice nunca apaga arquivos), reabrir recentes.
- Scanner com `.gitignore` + `.nexusignore`, exclusões padrão (`.git`, `node_modules`, `dist`, `build`, `target`, caches), limites de tamanho/profundidade, detecção de binários, arquivos sensíveis por nome e por conteúdo (registrados sem conteúdo), sem seguir symlinks/junctions para fora do escopo.
- Indexador incremental: fast-path por tamanho+mtime, hash blake3 fora de transação, chunks endereçados por conteúdo (dedup), FTS5 contentless com subpalavras de identificadores, lotes curtos, detecção de alteração durante a leitura, remoções e renomeações por hash, versão do parser registrada, `index_generation` e commit HEAD.
- Resource Governor: threads configuráveis, pausa, cancelamento, backoff, prioridade para interação; jobs órfãos marcados `interrupted` e retomáveis.
- Watcher com debounce + reconciliação periódica; eventos de progresso na UI.

Aceite: contagens exatas nas fixtures; alterar 1 arquivo num repositório sintético de 10 000 arquivos reprocessa exatamente 1; cancelamento medido; crash simulado no meio + retomada chega ao mesmo estado final; symlink/junction para fora do escopo ignorado.

## Fase 3 — Busca e Context Packs

- Ranking híbrido (bm25, caminho, símbolo, intenção por tipo de arquivo, recência, grau no grafo) com motivos legíveis por componente; expansão PT→EN de termos técnicos documentada.
- Trechos lidos do disco e verificados por hash ⇒ estado de atualidade por resultado; deduplicação por conteúdo; preview.
- Compilador de Context Pack: análise da tarefa, busca, expansão por dependências, testes relacionados, orçamento estimado de tokens, limitações sempre presentes; persistido com fontes e versão do índice; edição, cópia e exportação Markdown (cercas seguras + aviso de conteúdo não confiável) e JSON versionado.

Aceite: 100% das referências (caminho + linhas + hash) válidas por teste invariante; avaliação com ≥ 25 consultas medindo Recall@k, Precision@k e MRR comparando FTS puro × híbrido × contextual, com números reais publicados.

## Fase 4 — Memória e atualidade

- Memórias tipadas (decisão, convenção, requisito, restrição, bug, solução, dependência, tarefa, lição) com origem, data, estado (proposta → confirmada → arquivada/substituída), referências e histórico completo de revisões.
- Fontes que não sejam o usuário só criam propostas; nada sobrescreve decisões confirmadas sem revisão.
- Importação de ADRs como propostas.
- Detecção de: caminhos inexistentes citados em docs, símbolos removidos citados em docs, referências de memória com arquivo ausente ou hash alterado, memórias duplicadas, arquivos alterados desde a indexação. Cada alerta com fontes, motivo, data e ação recomendada; "requer revisão humana" quando indecidível.

## Fase 5 — MCP e CLI

- CLI completa: `projects`, `index`, `status`, `search`, `context`, `memory`, `freshness`, `diagnostics`, `mcp serve`, saída humana e `--json`, códigos de saída corretos.
- Servidor MCP stdio com as ferramentas `list_projects`, `get_project_overview`, `search_project`, `get_file_context`, `get_symbol_context`, `get_related_files`, `get_project_memory`, `build_context_pack`, `get_project_decisions`, `get_stale_context`, `get_index_status`, `refresh_project_index`, `get_context_sources`; negociação de versão conforme a especificação vigente; schemas de entrada/saída; validação de escopo; leitura somente leitura; atualização incremental limitada e auditada.
- Testes de sessão JSON-RPC, MCP Inspector e conexão real com Claude Code; guia de integração.

## Fase 6 — Inteligência avançada

Analisadores por linguagem com cobertura declarada (tree-sitter apenas se a avaliação justificar), grafo de dependências incremental com evidência por aresta, melhorias de ranking validadas contra a baseline da Fase 3, benchmarks reproduzíveis (indexação inicial/incremental, latência de busca e de pack, memória, CPU) com resultados reais.

## Fase 7 — Hardening e release

Auditoria de segurança, revisão do modelo de ameaças, testes de desempenho e recuperação, refinamento de UX, instalador final, CI completo, documentação final, demonstração reproduzível. Assinatura de código e atualização segura ficam marcadas como **pendentes** até estarem de fato configuradas.

## Riscos

| Risco                                             | Impacto                       | Mitigação                                                               |
| ------------------------------------------------- | ----------------------------- | ----------------------------------------------------------------------- |
| Tauri 3 alpha aparece como `latest`               | quebra de build               | `tauri = "2.12"` e plugins 2.x fixados; atualização manual e deliberada |
| TypeScript 7 incompatível com typescript-eslint 8 | lint quebrado                 | TypeScript 6.0.x fixado; reavaliar quando houver suporte                |
| `notify-debouncer-full` 0.8 ainda RC              | watcher instável              | usar 0.7 + reconciliação periódica                                      |
| Concorrência desktop/CLI/MCP no mesmo SQLite      | `SQLITE_BUSY`, inconsistência | WAL, busy_timeout, transações curtas, lock de job com heartbeat         |
| Falha no meio de migration                        | perda de dados                | transação por migration, backup prévio, recusa de downgrade             |
| Symlinks/junctions no Windows                     | sair do escopo autorizado     | não seguir links, checar reparse points e prefixo canônico              |
| Prompt injection em conteúdo indexado             | agente manipulado             | conteúdo rotulado como dado, cercas seguras, MCP somente leitura        |
| Segredos indexados                                | vazamento                     | exclusão por nome + detecção por conteúdo; nunca logar conteúdo         |
| E2E via CDP depende do WebView2                   | instabilidade                 | porta dedicada, timeouts explícitos, logs anexados                      |
| Instalador sem assinatura                         | alerta do SmartScreen         | documentado; assinatura na Fase 7                                       |
