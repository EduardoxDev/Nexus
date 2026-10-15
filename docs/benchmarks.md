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

### v0.2 — Go, Java e C# (2026-10-13, mesma máquina)

Mesmo teste, `main` e a branch da etapa rodados em sequência para comparar nas mesmas condições (a máquina estava mais lenta que no dia da tabela acima: só gerar os arquivos levou 44–51 s):

| Medida                              | `main` | Com Go/Java/C# |
| ----------------------------------- | ------ | -------------- |
| Indexação inicial (10 000 arquivos) | 13,2 s | 15,5 s         |
| Incremental sem mudanças            | 0,40 s | 0,40 s         |
| Incremental com 1 arquivo alterado  | 0,89 s | 0,91 s         |

A diferença na inicial está dentro da variação da máquina; as incrementais ficaram iguais. A resolução nova só roda para arquivos Go/Java/C#.

### v0.3 — Kotlin, PHP, Ruby, C e C++ (2026-10-14, mesma máquina)

Mesmo teste, com o `main` e esta branch alternados (main, branch, main, branch), já com os ajustes da revisão:

| Medida                                         | `main` (2 rodadas) | Com as linguagens novas (2 rodadas) |
| ---------------------------------------------- | ------------------ | ----------------------------------- |
| Indexação inicial (10 000 arquivos)            | 9,4 s / 9,8 s      | 8,5 s / 17,9 s                      |
| Incremental sem mudanças                       | 0,40 s / 0,42 s    | 0,51 s / 1,19 s                     |
| Incremental com 1 arquivo alterado             | 0,90 s / 0,94 s    | 0,97 s / 1,38 s                     |
| Análise em memória (10 000 arquivos, 1 thread) | 99 ms / 102 ms     | 144 ms / 161 ms                     |

A segunda rodada da branch ficou lenta em tudo, inclusive no que não mudou (indexação e incremental): a máquina estava ocupada. A análise em memória, porém, ficou mais lenta nas duas rodadas. Por isso medi de novo só a análise, isolada (3 passadas seguidas em cada execução, alternando `main` e branch):

| Execução | `main`            | Branch            |
| -------- | ----------------- | ----------------- |
| 1ª       | 99 / 101 / 95 ms  | 209 / 139 / 96 ms |
| 2ª       | 99 / 106 / 117 ms | 95 / 94 / 87 ms   |

Na 1ª execução da branch, as **duas primeiras passadas** saíram lentas (209 e 139 ms) e só a terceira voltou ao nível do `main`. Na 2ª execução, as três passadas da branch ficaram iguais ou abaixo do `main` (87–95 ms contra 99–117 ms). Leio isso como aquecimento logo depois de compilar (o binário recém-gerado), e o teste de escala mede uma passada só, justamente logo depois de compilar. Não dá para descartar de vez uma diferença pequena: se houver, ela não aparece depois do aquecimento. As linguagens novas não entram no caminho do TypeScript, exceto o percorredor de chaves comum, que ganhou a contagem de parênteses do cabeçalho; ela não mudou o tempo medido.

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

### Mapa do código (v0.3)

Medido em memória, numa thread (`project_map_scales_to_ten_thousand_files` em `tests/analysis.rs`, release): montar o mapa de 10 000 arquivos e 19 962 imports (graus, PageRank com 30 iterações e corte nos 150 nós mais centrais) levou **53 ms**; em debug, 307 ms. A leitura das arestas no banco vem antes e não entra nessa conta. O layout do desenho roda no app, sobre no máximo 500 nós.

### Variação do tempo de parede

Nesta máquina, o tempo de parede da indexação inicial é dominado por E/S. O antivírus examina arquivos recém-criados em `%TEMP%`. No mesmo dia, a indexação inicial do teste de escala variou de 5,8 a 8,7 s com o código da Fase 6, e o **commit da Fase 5** levou 32 s numa execução (3,6 s na véspera). Por isso a comparação entre fases usa o custo de CPU isolado acima, não o tempo de parede.

## Pendentes

Medições em Linux/macOS e com repositórios reais grandes (> 100 mil arquivos); resolução incremental do grafo por arquivo.
