# Benchmarks

Medições reais, registradas com data, máquina e comando. Não são metas.

## Indexação — repositório sintético de 10 000 arquivos

Comando:

```bash
cargo test -p nexus-core --release --test scale -- --ignored --nocapture
```

Conjunto: 10 000 arquivos TypeScript sintéticos (~300 B cada, 3,0 MiB no total) em 100 pastas, gerados pelo próprio teste (`crates/nexus-core/tests/scale.rs`).

Máquina: Intel Core i3-10100F (4 núcleos / 8 threads), Windows 11 Pro, SSD, Rust 1.99 `--release`. Indexador com 4 threads (padrão: metade dos núcleos lógicos).

| Medida (3 execuções em 2026-10-11)   | Resultado                                                                            |
| ------------------------------------ | ------------------------------------------------------------------------------------ |
| Indexação inicial (10 000 arquivos)  | 2,9 s · 3,1 s · 4,1 s                                                                |
| Incremental sem mudanças             | 0,40 s · 0,40 s · 0,44 s                                                             |
| Incremental com 1 arquivo alterado   | 0,39 s · 0,39 s · 0,43 s — **exatamente 1 arquivo reprocessado** (asserção do teste) |
| Banco após indexar (última execução) | 7,4 MiB + WAL de 4,8 MiB (fonte: 3,0 MiB)                                            |

Observações:

- O conjunto sintético tem vocabulário muito diverso (identificadores únicos por arquivo), o que infla o índice FTS. No repositório real abaixo, o banco ficou com ~1,3× o tamanho do conteúdo indexado.
- O WAL acima é o arquivo ainda aberto durante o teste; ao fechar o banco ele é incorporado e removido (testado em `closing_the_db_checkpoints_and_removes_the_wal`).
- O tempo da passada incremental é dominado pela varredura de diretórios e pela comparação de metadados.
- Gerar os 10 000 arquivos levou ~34 s (fora da medição).

## Repositório real

`nexus projects add <repo> --index` sobre o repositório do NEXUS (2026-10-11, build de depuração):

| Medida                     | Resultado                                                     |
| -------------------------- | ------------------------------------------------------------- |
| Arquivos encontrados       | 255 (235 indexados, 20 ignorados — imagens e ícones binários) |
| Conteúdo indexado          | 916 KiB, 27 250 linhas                                        |
| Tempo da indexação inicial | 0,5 s                                                         |
| Segunda passada            | 0 processados, todos inalterados                              |
| Banco após fechar          | 1,1 MiB (`nexus.db`; sem WAL residual)                        |

## Busca e Context Packs (10 000 arquivos)

Mesmo teste de escala (`tests/scale.rs`, release), após indexar: 24 buscas (8 consultas × 3 rodadas) e 3 Context Packs.

| Medida (2026-10-11)                             | Resultado                                                                                  |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------ |
| Busca híbrida                                   | p50 37,0 ms · p95 44,1 ms                                                                  |
| Context Pack (compilar + gravar + materializar) | 77–113 ms                                                                                  |
| Indexação inicial nesta execução                | 7,9 s (variação em relação às execuções anteriores, 2,9–4,1 s, com a máquina mais ocupada) |

Na fixture de avaliação (47 arquivos): busca p50 2,5 ms; Context Pack p50 7,9 ms — ver [evaluation.md](evaluation.md).

## Atualidade (Fase 4)

| Medida (2026-10-12, release)                                               | Resultado                                                                                    |
| -------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Verificação completa, 10 000 arquivos sem documentos (`tests/scale.rs`)    | 315–316 ms (3 execuções) — dominada pelo `stat` de cada arquivo indexado                     |
| Incremental com 1 arquivo alterado, **incluindo** a verificação automática | 0,82 s (antes da Fase 4: 0,39–0,43 s). Sem mudanças no índice a verificação não roda: 0,39 s |
| Repositório do NEXUS (36 documentos Markdown, debug)                       | 46–56 ms                                                                                     |

Nesta mesma execução: busca p50 48,6 ms · p95 54,0 ms; Context Pack 99–152 ms; indexação inicial 3,6 s.

### Precisão dos alertas de documentos

| Repositório                                                    | Alertas de documento | Avaliação                                                                                             |
| -------------------------------------------------------------- | -------------------- | ----------------------------------------------------------------------------------------------------- |
| 4 fixtures (`saas-app`, `ts-webapp`, `py-service`, `rust-lib`) | 0                    | teste `fixtures_have_no_document_alerts`                                                              |
| NEXUS (este repositório), 1ª versão da heurística              | 17                   | 16 falsos positivos: caminhos relativos a subprojeto (`tests/scale.rs`), pastas de execução (`logs/`) |
| NEXUS, heurística final                                        | 1                    | `test_service.py` citado como exemplo de convenção — marcado "requer revisão humana"                  |

Reproduzir: `NEXUS_FRESHNESS_REPO=<pasta> cargo test -p nexus-core --test freshness -- --ignored --nocapture`.

## Benchmarks reproduzíveis (Fase 6)

`powershell -ExecutionPolicy Bypass -File scripts/bench.ps1 -Files 10000 -Runs 5` compila a CLI em release, gera um repositório TypeScript sintético e determinístico (`crates/nexus-core/examples/synthetic_repo.rs`: 100 módulos por pacote, cada um importando o anterior e um utilitário comum) e mede cada etapa no **processo real da CLI**: tempo de parede, CPU (usuário + sistema) e pico do working set, amostrado a cada 10 ms (processos curtos podem ter o pico subestimado). A sessão MCP é conferida: o script falha se a busca não devolver resultado.

Execução de 2026-10-12, Intel Core i3-10100F (4 núcleos/8 threads), 8 GiB, Windows 11:

| Etapa                                                   | Amostras | Parede (ms, mediana) | CPU (ms, mediana) | Pico de memória (MiB) |
| ------------------------------------------------------- | -------- | -------------------- | ----------------- | --------------------- |
| Indexação inicial (10 000 arquivos)                     | 1        | 6 823                | 7 625             | 38,4                  |
| Incremental sem mudanças                                | 5        | 469                  | 422               | 21,2                  |
| Incremental com 1 arquivo alterado                      | 5        | 997                  | 953               | 35,2                  |
| Reindexação completa (`--full`, conteúdo reaproveitado) | 1        | 1 499                | 2 516             | 25,3                  |
| Busca (processo completo da CLI)                        | 20       | 118                  | 94                | 20,6                  |
| Context Pack (compilar + exportar)                      | 5        | 288                  | 266               | 20,5                  |
| `nexus symbol`                                          | 5        | 61                   | 31                | 15,1                  |
| `nexus deps <arquivo>`                                  | 5        | 62                   | 47                | 15,0                  |
| Sessão MCP com 1 busca                                  | 5        | 122                  | 94                | 20,7                  |

Grafo: 19 997 arestas, todas internas e resolvidas, montado em 257 ms. Banco: 23,1 MiB.

Os tempos da CLI incluem iniciar o processo e abrir o banco (~40–60 ms).

### Custo da análise de código

Medido em memória, numa thread, sem disco (`tests/scale.rs`, release): analisar os 10 000 arquivos sintéticos (30 000 declarações) levou **96–99 ms**, contra **199–203 ms** para dividir em chunks e montar o texto do FTS. A análise custa cerca de metade do que o indexador já fazia por arquivo. A reconstrução do grafo (~250 ms) e a verificação de atualidade (~320 ms) rodam na finalização de cada indexação com mudanças, o que explica o "1 arquivo alterado" perto de 0,9–1 s.

### Variação do tempo de parede

Nesta máquina, o tempo de parede da indexação inicial é dominado por E/S. O antivírus examina arquivos recém-criados em `%TEMP%`. No mesmo dia, a indexação inicial do teste de escala variou de 5,8 a 8,7 s com o código da Fase 6, e o **commit da Fase 5** levou 32 s numa execução (3,6 s na véspera). Por isso a comparação entre fases usa o custo de CPU isolado acima, não o tempo de parede.

## Pendentes

Medições em Linux/macOS e com repositórios reais grandes (> 100 mil arquivos); resolução incremental do grafo por arquivo.
