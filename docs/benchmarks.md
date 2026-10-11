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

## Pendentes

Consumo de memória e uso de CPU durante a indexação (Fase 6).
