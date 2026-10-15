# Avaliação da recuperação

Mede se a busca e os Context Packs encontram as fontes certas. **Não** mede produtividade de pessoas ou agentes — isso exigiria um experimento com usuários, ainda não feito.

## Como reproduzir

```bash
cargo test -p nexus-core --release --test evaluation -- --nocapture
```

O teste indexa uma cópia de `tests/fixtures/saas-app` (47 arquivos: API TypeScript com autenticação, cobrança, checkout, pedidos, notificações, React, migrations SQL, testes, 3 ADRs, guia de contribuição), roda cada tarefa nos três modos e grava `target/tmp/evaluation-results.json`.

## Conjuntos de tarefas

| Conjunto  | Arquivo                         | Tarefas                 | Uso                                                                               |
| --------- | ------------------------------- | ----------------------- | --------------------------------------------------------------------------------- |
| Principal | `tests/evaluation/tasks.json`   | 28 (português e inglês) | medição e ajuste do ranking                                                       |
| Reservado | `tests/evaluation/holdout.json` | 12                      | escrito **depois** dos ajustes e **nunca** usado para ajustar; mede generalização |

Cada tarefa lista as fontes que uma pessoa que conhece o projeto consideraria necessárias (`relevant`), definidas antes da medição.

## Estratégias comparadas

1. **Busca simples** — bm25 do FTS5 apenas com os termos digitados (sem expansão, sem caminho, sem prioridades).
2. **Busca híbrida** — bm25 com expansão PT→EN e subpalavras + correspondência de caminho + cobertura de termos + declarações + intenção por tipo de arquivo + recência (ver [architecture.md](architecture.md#11-busca-e-context-packs-fase-3)).
3. **Recuperação contextual** — lista de fontes do Context Pack (busca híbrida + decisões + testes correspondentes + convenções + manifestos, ordenados por papel e intenção).

## Resultados (2026-10-11, build `--release`)

### Conjunto principal (28 tarefas)

| Estratégia             | Recall@5 | Precision@5 | Recall@10 | Precision@10 | MRR   |
| ---------------------- | -------- | ----------- | --------- | ------------ | ----- |
| Busca simples          | 0,682    | 0,236       | 0,748     | 0,139        | 0,668 |
| Busca híbrida          | 0,918    | 0,336       | 0,961     | 0,182        | 0,958 |
| Recuperação contextual | 0,897    | 0,329       | 0,952     | 0,186        | 0,964 |

### Conjunto reservado (12 tarefas, sem ajuste)

| Estratégia             | Recall@5 | Precision@5 | Recall@10 | Precision@10 | MRR   |
| ---------------------- | -------- | ----------- | --------- | ------------ | ----- |
| Busca simples          | 0,708    | 0,167       | 0,792     | 0,092        | 0,616 |
| Busca híbrida          | 1,000    | 0,250       | 1,000     | 0,125        | 0,833 |
| Recuperação contextual | 1,000    | 0,250       | 1,000     | 0,125        | 0,958 |

### Referências e latência

| Medida                                                            | Principal     | Reservado                        |
| ----------------------------------------------------------------- | ------------- | -------------------------------- |
| Referências válidas (caminho + linhas + hash conferidos no disco) | 644/644       | 278/278                          |
| Context Packs com fontes                                          | 28/28         | 12/12                            |
| Busca híbrida (p50 / p95)                                         | 2,5 / 4,8 ms  | 3,1 / 6,9 ms                     |
| Context Pack (p50 / p95)                                          | 7,9 / 13,5 ms | 13,4 / 244 ms (um valor isolado) |

Em escala (10 000 arquivos sintéticos, ver [benchmarks.md](benchmarks.md)): busca p50 37 ms / p95 44 ms; Context Pack 77–113 ms.

## Fase 6 — ranking com o grafo (2026-10-12, `--release`)

Três mudanças, medidas uma de cada vez **só no conjunto principal**. O conjunto reservado foi medido uma única vez, no fim, sem ajuste depois.

| Mudança                                                               | Busca híbrida (principal)                          | Context Pack (principal)           |
| --------------------------------------------------------------------- | -------------------------------------------------- | ---------------------------------- |
| Baseline (= Fase 3)                                                   | R@5 0,918 · P@5 0,336 · R@10 0,961 · MRR 0,958     | R@5 0,897 · R@10 0,952 · MRR 0,964 |
| 1. Declarações da análise (tipo e linha nos motivos)                  | igual                                              | igual                              |
| 2. Centralidade no grafo (peso ≤ 0,05, fixado antes de medir)         | **R@5 0,936** · P@5 0,343 · R@10 0,961 · MRR 0,958 | R@10 **0,961** · P@10 0,189        |
| 3. Expansão 1-hop nos packs (testes que importam, até 4 dependências) | igual                                              | igual                              |

Resultado final:

| Conjunto       | Estratégia             | Recall@5 | Precision@5 | Recall@10 | Precision@10 | MRR                   |
| -------------- | ---------------------- | -------- | ----------- | --------- | ------------ | --------------------- |
| Principal (28) | Busca híbrida          | 0,936    | 0,343       | 0,961     | 0,182        | 0,958                 |
| Principal (28) | Recuperação contextual | 0,897    | 0,329       | 0,961     | 0,189        | 0,964                 |
| Reservado (12) | Busca híbrida          | 1,000    | 0,250       | 1,000     | 0,125        | **0,875** (era 0,833) |
| Reservado (12) | Recuperação contextual | 1,000    | 0,250       | 1,000     | 0,125        | 0,958                 |

Referências válidas: 659/659 (principal) e 282/282 (reservado). Latência na fixture: busca p50 2,8 ms; pack p50 7,0 ms.

As mudanças 1 e 3 são neutras nas métricas. Foram mantidas porque deixam os motivos e as fontes mais precisos (tipo e linha da declaração; "Importado por X na linha N"), sem custo mensurável no ranking.

## Análise de código (Fase 6)

`cargo test -p nexus-core --test analysis -- --nocapture` compara os analisadores com `tests/evaluation/analysis.json`, um gabarito escrito lendo cada arquivo das fixtures (4 na Fase 6, 12 desde a v0.3), **antes** de olhar a saída dos analisadores, e restrito à cobertura declarada.

| Linguagem             | Declarações (precisão / revocação) | Imports com destino (precisão / revocação) |
| --------------------- | ---------------------------------- | ------------------------------------------ |
| TypeScript/JavaScript | 1,000 / 1,000 (76)                 | 1,000 / 1,000 (73)                         |
| Python                | 1,000 / 1,000 (14)                 | 1,000 / 1,000 (7)                          |
| Rust                  | 1,000 / 1,000 (8)                  | 1,000 / 1,000 (8)                          |

As fixtures são pequenas e limpas, então esse resultado é otimista. Medição no repositório do NEXUS (215 arquivos analisados, 1 309 arestas internas), com `NEXUS_ANALYSIS_REPO=<pasta> cargo test -p nexus-core --test analysis -- --ignored --nocapture`:

- **Revocação contra uma busca independente** (regex de declarações exportadas/públicas de nível superior, fora de strings; `mod x;` excluído por ser import): 727/727.
- **Amostra de declarações** (1 a cada 7 arquivos), conferida à mão: 39/39 com nome, tipo, linha e contêiner corretos.
- **Amostra de arestas resolvidas** (1 a cada 45), conferida à mão: **26/30 na primeira medição**. As 4 erradas eram `use super::*` dentro de `mod tests { … }` inline, apontando para o módulo pai em vez do próprio arquivo. Depois da correção: 29/29, e 35 arestas erradas sumiram do grafo.
- **Imports sem destino**: 3 na primeira medição (`use std::…` em um crate só com `main.rs`, cuja raiz não era achada). Depois da correção: 0.

## Histórico honesto das medições

1. **Primeira medição** (antes de qualquer ajuste): busca híbrida Recall@5 0,846, MRR 0,866; contextual Recall@5 0,799, MRR 0,773.
2. As falhas por tarefa apontaram causas **gerais**: termos comuns ausentes no dicionário PT→EN (`tabela`, `dinheiro`, `formatar`, `salvar`, `tempo`…), documentação em português vencendo código em perguntas sobre código, e packs que ignoravam a intenção da tarefa (decisões/testes pedidos explicitamente). Corrigi essas causas — não regras específicas por tarefa.
3. Como os ajustes foram guiados pelo conjunto principal, há **risco de sobreajuste**. Por isso o conjunto reservado foi escrito depois e nunca usado para ajustar: a busca híbrida ganhou da simples também ali (MRR 0,833 × 0,616).
4. Ajustes posteriores vieram de uso real da interface (não do conjunto): flexões em português (`validada` → `validar`), desempate de trechos e declarações pela cobertura de conceitos, limite de documentação genérica nos packs e testes procurados também para arquivos relacionados. As métricas permaneceram estáveis.

## v0.2 — Go, Java e C# (2026-10-13)

Gabarito conferido à mão em três fixtures novas (`tests/evaluation/analysis.json`), medido por `cargo test -p nexus-core --test analysis -- --nocapture`:

| Linguagem | Símbolos: precisão | Símbolos: revocação | Imports resolvidos: precisão | Imports resolvidos: revocação |
| --------- | ------------------ | ------------------- | ---------------------------- | ----------------------------- |
| Go        | 1,000 (14/14)      | 1,000               | 1,000 (12/12)                | 1,000                         |
| Java      | 1,000 (19/19)      | 1,000               | 1,000 (8/8)                  | 1,000                         |
| C#        | 1,000 (17/17)      | 1,000               | 1,000 (7/7)                  | 1,000                         |

As linguagens anteriores continuaram em 1,000. Limitação: as fixtures são pequenas e escritas por mim; código real com geração de código, text blocks ou strings verbatim desbalanceadas pode errar (ver cobertura declarada em `analysis::coverage()`).

## v0.3 — Kotlin, PHP, Ruby, C e C++ (2026-10-14)

Gabarito conferido à mão em cinco fixtures novas (`kotlin-app`, `php-app`, `ruby-app`, `c-lib`, `cpp-app`), no mesmo teste:

| Linguagem | Símbolos: precisão | Símbolos: revocação | Imports e destinos: precisão | Imports e destinos: revocação |
| --------- | ------------------ | ------------------- | ---------------------------- | ----------------------------- |
| Kotlin    | 1,000 (16/16)      | 1,000               | 1,000 (4/4)                  | 1,000                         |
| PHP       | 1,000 (14/14)      | 1,000               | 1,000 (5/5)                  | 1,000                         |
| Ruby      | 1,000 (15/15)      | 1,000               | 1,000 (6/6)                  | 1,000                         |
| C/C++     | 1,000 (22/22)      | 1,000               | 1,000 (13/13)                | 1,000                         |

As colunas de imports contam o import **e o destino** certo, inclusive "não resolvido" quando é o certo (como o `Logger` do PHP) e "externo".

As fixtures cobrem os casos difíceis de propósito:

- PHP: import com PSR-4, `use` em grupo e um nome que não existe no projeto, que fica **não resolvido** em vez de inventado.
- Ruby: `require` de pasta local × gem.
- C/C++: `#include` relativo, pela pasta `include/` e com `<…>` de arquivo do projeto; métodos definidos fora da classe (`Account::balance`).

As linguagens anteriores continuaram em 1,000.

**Código real (só C/C++):** conferi as declarações de 33 arquivos C/C++ de um repositório real de terceiros (26 `.c`, 6 `.cc`, 1 `.hpp`) contra o código-fonte. Não achei declaração faltando nem sobrando. As diferenças em relação a um `grep` de conferência eram erros do `grep`: corpo na mesma linha, `__attribute__`, `struct` sem nome de função.

Kotlin, PHP e Ruby **não** foram conferidos em código real nesta etapa: só nas fixtures.

## Falhas conhecidas (conjunto principal)

| Tarefa                                     | Problema                                                                                                            |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------- |
| `auth-where` ("Onde fica a autenticação?") | O ADR de sessões (em português, com "autenticação" no texto) fica em 1º; o código aparece a partir do 3º. MRR 0,33. |
| `session-expiry`                           | O teste de sessão aparece antes de `session.ts`. MRR 0,50.                                                          |
| `checkout-coupon-change`, `google-login`   | Recall@5 0,5–0,6: parte das fontes esperadas fica entre a 6ª e a 10ª posição.                                       |

## Limitações da avaliação

- Uma única fixture pequena (47 arquivos), escrita por mim: os números indicam tendência, não garantia para repositórios reais.
- Relevância binária definida por uma pessoa; não há graus de relevância (nDCG) nem múltiplos avaliadores.
- Precision@k é limitada pelo número de fontes relevantes (muitas tarefas têm 1–2), então valores baixos de precisão são esperados.
- Não há comparação com embeddings — por decisão (ADR-0006), só entram se superarem esta baseline.
