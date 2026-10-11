# Modelo de ameaças

> Documento vivo. Revisado ao fim de cada fase. Estado de cada mitigação: **implementada**, **parcial** ou **planejada (Fase N)**.

## 1. Ativos

| Ativo                                                | Por que importa                                              |
| ---------------------------------------------------- | ------------------------------------------------------------ |
| Código-fonte e documentação dos projetos autorizados | propriedade intelectual; pode conter segredos                |
| Banco `nexus.db` (índice, memória, histórico)        | revela estrutura, caminhos, decisões técnicas                |
| Logs locais                                          | podem revelar caminhos e erros                               |
| Integridade do contexto entregue a agentes           | contexto adulterado induz agentes a erros ou ações perigosas |
| Máquina do usuário                                   | o NEXUS não pode virar vetor de execução arbitrária          |

## 2. Fronteiras de confiança

1. **WebView ↔ core Rust** (IPC do Tauri).
2. **Conteúdo do repositório ↔ core** (arquivos são dados não confiáveis).
3. **Cliente MCP / CLI ↔ core** (entradas externas, possivelmente de um agente manipulado).
4. **Disco local ↔ outros processos/usuários** do sistema operacional.
5. **Cadeia de suprimentos** (crates, pacotes npm, toolchains).

## 3. Ameaças e mitigações (STRIDE)

| #   | Fronteira          | Ameaça                                                                | Mitigação                                                                                                                                                      | Estado                                                        |
| --- | ------------------ | --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| T1  | WebView↔core       | Script injetado no webview invoca APIs nativas amplas                 | Sem plugins `fs`/`shell`/`http` expostos; `withGlobalTauri: false`; capabilities listam só comandos do app; CSP sem origens remotas                            | implementada (Fase 1)                                         |
| T2  | WebView↔core       | Entradas maliciosas em comandos (chaves arbitrárias, limites enormes) | Allowlist de chaves de configuração com validação de valor; limites numéricos com teto (`list_operations ≤ 200`)                                               | implementada (Fase 1)                                         |
| T3  | WebView↔core       | Comando genérico que abre/lê caminhos arbitrários                     | Não existe; `reveal_data_dir` só abre o diretório de dados resolvido no Rust                                                                                   | implementada (Fase 1)                                         |
| T4  | Repositório↔core   | Path traversal via nomes de arquivo/links                             | Todos os caminhos relativos ao root canônico; rejeição de `..` e de caminhos fora do prefixo canônico                                                          | planejada (Fase 2)                                            |
| T5  | Repositório↔core   | Symlinks/junctions apontando para fora do escopo                      | Walker não segue links; reparse points ignorados; verificação de prefixo canônico                                                                              | planejada (Fase 2)                                            |
| T6  | Repositório↔core   | Arquivos enormes, profundamente aninhados, binários, compactados      | Limite de tamanho (1 MiB padrão), limite de profundidade, detecção de binário, compactados não são expandidos                                                  | planejada (Fase 2)                                            |
| T7  | Repositório↔core   | Segredos indexados (.env, chaves, tokens)                             | Exclusão por nome + detecção de padrões de conteúdo; registrado como `skipped_sensitive` sem conteúdo; nunca logar conteúdo                                    | planejada (Fase 2)                                            |
| T8  | Repositório↔agente | Prompt injection em código/docs repassado a agentes                   | Conteúdo sempre rotulado como dado não confiável; exportação Markdown com cercas mais longas que qualquer sequência de crases do conteúdo; MCP somente leitura | planejada (Fases 3/5)                                         |
| T9  | MCP/CLI↔core       | Requisições malformadas ou fora de escopo                             | Validação por schema, limites, projetos identificados por ID registrado, caminhos resolvidos só dentro do root                                                 | planejada (Fase 5)                                            |
| T10 | MCP/CLI↔core       | Agente força reindexações em loop (DoS)                               | `refresh_project_index` incremental, com rate limit e auditoria em `operations`                                                                                | planejada (Fase 5)                                            |
| T11 | MCP↔core           | Agente sobrescreve decisões confirmadas                               | Fontes não-usuário só criam propostas; confirmação exige ação do usuário                                                                                       | planejada (Fase 4)                                            |
| T12 | Disco local        | Corrupção do índice (crash, disco cheio)                              | WAL, transações curtas, verificação de integridade sob demanda, índice reconstruível a partir dos arquivos                                                     | parcial (Fase 1: integridade; Fase 2: reconstrução)           |
| T13 | Disco local        | Falha durante migration                                               | Transação por migration, backup prévio automático, recusa de downgrade, checksum de migrations                                                                 | implementada (Fase 1)                                         |
| T14 | Disco local        | Outros processos do mesmo usuário leem `nexus.db`                     | **Limite conhecido**: o NEXUS não criptografa o banco; a proteção é a ACL padrão do perfil do usuário (`%LOCALAPPDATA%`). Documentado em `privacy.md`          | aceito/documentado                                            |
| T15 | Disco local        | Logs vazando conteúdo                                                 | Política de redação: logs só com metadados (caminhos, contagens, durações, códigos de erro)                                                                    | implementada (Fase 1)                                         |
| T16 | Supply chain       | Dependência comprometida                                              | Lockfiles versionados, versões fixadas, dependências justificadas, `cargo audit`/`pnpm audit` no CI                                                            | parcial (Fase 1: lockfiles; auditoria automatizada na Fase 7) |
| T17 | Exportação         | Injeção de conteúdo em Markdown/JSON exportado                        | Escape/cercas seguras; JSON serializado (nunca concatenado)                                                                                                    | planejada (Fase 3)                                            |
| T18 | Distribuição       | Binário adulterado                                                    | Assinatura de código e updater assinado                                                                                                                        | planejada (Fase 7) — **não configurado**                      |

## 4. Fora de escopo (por ora)

- Atacante com privilégios de administrador ou com a sessão do usuário desbloqueada.
- Sincronização em nuvem (não existe; nenhum dado sai da máquina).
