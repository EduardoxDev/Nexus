# Roadmap

Cada fase termina com aplicação executável e testes aprovados. Critérios de aceitação são verificáveis; metas de desempenho só são registradas depois de medidas.

| Fase | Tema                         | Estado    |
| ---- | ---------------------------- | --------- |
| 0    | Reconhecimento e arquitetura | concluída |
| 1    | Fundação desktop             | concluída |
| 2    | Indexação de projetos        | concluída |
| 3    | Busca e Context Packs        | concluída |
| 4    | Memória e atualidade         | concluída |
| 5    | MCP e CLI                    | concluída |
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

Critérios de aceitação (verificados em 2026-10-11):

- [x] Contagens exatas nas fixtures, inclusive com casos adversariais gerados no teste (`initial_index_has_exact_counts`).
- [x] Alterar 1 arquivo num repositório sintético de 10 000 arquivos reprocessa exatamente 1 (`tests/scale.rs`, release: 0,39–0,43 s; inicial 2,9–4,1 s — ver [benchmarks.md](benchmarks.md)).
- [x] Cancelamento medido: ~4 ms do pedido até o job encerrado, lotes gravados preservados e retomada chegando ao mesmo estado de uma indexação limpa (`cancel_keeps_committed_batches_and_resume_reaches_clean_state`).
- [x] Queda simulada no meio + retomada chega ao mesmo estado final; job órfão marcado como interrompido (`crash_mid_run_is_recovered`).
- [x] Junction para fora do escopo não é seguida (`links_outside_the_project_are_not_followed`, junction real do Windows).
- [x] Sensíveis nunca chegam ao FTS nem têm hash guardado; binários, grandes e outra codificação registrados com motivo.
- [x] Renomeação detectada e conteúdo reaproveitado; duplicatas compartilham conteúdo; remoções limpam chunks e FTS.
- [x] Pausa/retomada, trava entre processos, job de processo morto substituído.
- [x] Desktop: progresso ao vivo, monitor de arquivos reindexando sozinho e remoção sem apagar arquivos — provados por E2E no app real.

Fora do escopo desta fase (seguem no roadmap): símbolos e imports (Fase 3/6); busca exposta na interface (Fase 3). O "backoff quando o sistema estiver ocupado" ficou limitado a um job por vez, threads configuráveis e pausa manual — sem medição automática de carga do sistema.

## Fase 3 — Busca e Context Packs

- Ranking híbrido (bm25, caminho, símbolo, intenção por tipo de arquivo, recência, grau no grafo) com motivos legíveis por componente; expansão PT→EN de termos técnicos documentada.
- Trechos lidos do disco e verificados por hash ⇒ estado de atualidade por resultado; deduplicação por conteúdo; preview.
- Compilador de Context Pack: análise da tarefa, busca, expansão por dependências, testes relacionados, orçamento estimado de tokens, limitações sempre presentes; persistido com fontes e versão do índice; edição, cópia e exportação Markdown (cercas seguras + aviso de conteúdo não confiável) e JSON versionado.

Critérios de aceitação (verificados em 2026-10-11):

- [x] 100% das referências válidas por teste invariante: 644/644 (conjunto principal) e 278/278 (reservado) — caminho, linhas e hash conferidos no disco.
- [x] Avaliação com 28 tarefas (+12 reservadas) medindo Recall@k, Precision@k e MRR para busca simples × híbrida × contextual; números reais em [evaluation.md](evaluation.md). Busca híbrida: MRR 0,958 (simples: 0,668); no conjunto reservado, 0,833 × 0,616.
- [x] Resultados com caminho, trecho, motivo, linhas, data de indexação e estado de atualidade; deduplicação por conteúdo; prévia restrita a arquivos indexados.
- [x] Context Packs com objetivo, resumo, arquivos, trechos, convenções, decisões, dependências, testes, riscos, contradições (pendente: Fase 4), fontes, data/atualidade e limitações; persistidos só como referências; exportação Markdown/JSON com exclusões e notas.
- [x] Desktop (Busca, Context Packs) e CLI (`search`, `context`) com testes de componente, CLI e E2E no app real.

Decisões em [ADR-0010](decisions/ADR-0010-busca-explicavel-e-packs-por-referencia.md). Pendências desta área: grafo de dependências e símbolos (Fase 6); contradições entregues na Fase 4.

## Fase 4 — Memória e atualidade

- Memórias tipadas (decisão, convenção, requisito, restrição, bug, solução, dependência, tarefa, lição) com origem, data, estado (proposta → confirmada → arquivada/substituída), referências e histórico completo de revisões.
- Fontes que não sejam o usuário só criam propostas; nada sobrescreve decisões confirmadas sem revisão.
- Importação de ADRs como propostas.
- Detecção de: caminhos inexistentes citados em docs, símbolos removidos citados em docs, referências de memória com arquivo ausente ou hash alterado, memórias duplicadas, arquivos alterados desde a indexação. Cada alerta com fontes, motivo, data e ação recomendada; "requer revisão humana" quando indecidível.

Critérios de aceitação (verificados em 2026-10-12):

- [x] Nove tipos de memória com origem, datas, estados `proposed → confirmed → archived | superseded`, referências por hash e histórico integral de revisões (`tests/memory.rs`).
- [x] Fontes que não são o usuário (importação de ADRs, agentes) só criam ou ajustam propostas; confirmar, arquivar e editar confirmadas exige o usuário — testado para `agent`, `adr_import` e `system`.
- [x] ADRs importados como propostas, de forma idempotente; ADR alterado depois da confirmação vira proposta de substituição sem tocar na decisão confirmada (`changed_adr_never_overwrites_a_confirmed_decision`).
- [x] Detecção de caminhos inexistentes, símbolos removidos, referências de memória alteradas/movidas/ausentes, origem alterada, decisão substituída na origem, duplicatas e contradições entre memórias, arquivos alterados desde a indexação — cada alerta com fontes (caminho e linha), motivo, datas, recomendação e "requer revisão humana" (`tests/freshness.rs`).
- [x] Alertas estáveis: dispensado continua dispensado; corrigido é resolvido; reaparecimento reabre.
- [x] Nenhum alerta de documento nas 4 fixtures; no repositório do NEXUS, 1 alerta (exemplo ilustrativo, marcado para revisão) — ver [benchmarks.md](benchmarks.md).
- [x] Context Packs incluem só memórias confirmadas (por referência, com aviso se editadas depois) e a seção "Contradições e alertas pendentes" deixou de ser placeholder; JSON `schemaVersion` 2 e planos v1 ainda carregam.
- [x] Desktop (Memória, Atualidade, packs, aviso ao remover projeto, paleta) e CLI (`memory`, `freshness`, código de saída 5 para conflito), com testes de componente, CLI e E2E no app real (11/11).

Decisões em [ADR-0011](decisions/ADR-0011-memoria-revisada-e-alertas-estaveis.md). Limitações: nomes citados que nunca existiram no código não geram alerta; contradições são heurísticas (sem análise semântica); a CLI age como usuário (agentes com shell herdam esse poder até o MCP da Fase 5 usar o papel `agent`); remover um projeto apaga sua memória (com aviso, sem lixeira); a verificação automática acrescenta ~0,3 s por indexação com mudanças em 10 000 arquivos.

## Fase 5 — MCP e CLI

- CLI completa: `projects`, `index`, `status`, `search`, `context`, `memory`, `freshness`, `diagnostics`, `mcp serve`, saída humana e `--json`, códigos de saída corretos.
- Servidor MCP stdio com as ferramentas `list_projects`, `get_project_overview`, `search_project`, `get_file_context`, `get_symbol_context`, `get_related_files`, `get_project_memory`, `build_context_pack`, `get_project_decisions`, `get_stale_context`, `get_index_status`, `refresh_project_index`, `get_context_sources`; negociação de versão conforme a especificação vigente; schemas de entrada/saída; validação de escopo; leitura somente leitura; atualização incremental limitada e auditada.
- Testes de sessão JSON-RPC, MCP Inspector e conexão real com Claude Code; guia de integração.

Critérios de aceitação (verificados em 2026-10-12):

- [x] Revisão vigente verificada antes de implementar (2026-07-28); servidor dual-era: `server/discover`, `_meta` por requisição, `-32022` para versão não suportada e `initialize` para 2025-11-25/2025-06-18/2025-03-26 ([ADR-0005](decisions/ADR-0005-mcp-implementado-a-mao.md)).
- [x] As 13 ferramentas com `inputSchema` fechado, `outputSchema` e `structuredContent`; **todas** as saídas validadas contra o próprio esquema (`every_tool_output_matches_its_output_schema`).
- [x] Escopo: só projetos autorizados, só arquivos indexados (sensíveis recusados), caminhos fora do projeto recusados, argumentos validados; erros de uso como `isError`, erros de protocolo como JSON-RPC.
- [x] Somente leitura, exceto `refresh_project_index` (incremental, 1/30 s por projeto, até 120 s, trava entre processos) e `build_context_pack` (só referências no banco do NEXUS).
- [x] Auditoria em `operations` (`mcp.session`, `mcp.tool`) sem argumentos (`tool_calls_are_audited_without_arguments`), visível ao vivo na tela Integrações.
- [x] CLI completa: `projects`, `index`, `status`, `search`, `context`, `memory`, `freshness`, `diagnostics`, `mcp serve`, `mcp tools`, com `--json` e códigos de saída 0–5.
- [x] Testes de sessão in-process (6), binário real por stdio (`mcp_serve_speaks_json_rpc_over_stdio`) e E2E com o app aberto no mesmo banco (12/12 no total).
- [x] Guia de integração ([mcp.md](mcp.md)) e tela Integrações com o comando pronto para o Claude Code.
- [ ] MCP Inspector: não executado (exige baixar o pacote `@modelcontextprotocol/inspector` pelo npm; pendente de autorização).
- [ ] Conversa real com o Claude Code: `claude mcp list` (2.1.296) reconhece o servidor e o deixa aguardando aprovação; a chamada pelo modelo ainda não foi feita porque envia o resultado das ferramentas à API do provedor — pendente de autorização do usuário.

Limitações: `get_symbol_context` e `get_related_files` são heurísticos (sem análise por linguagem nem grafo de imports — Fase 6); o MCP não grava memória (propostas de agentes ficam para uma fase futura); a CLI não vem no instalador (Fase 7); sem resources, prompts nem transporte HTTP.

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
