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

## Histórico honesto das medições

1. **Primeira medição** (antes de qualquer ajuste): busca híbrida Recall@5 0,846, MRR 0,866; contextual Recall@5 0,799, MRR 0,773.
2. As falhas por tarefa apontaram causas **gerais**: termos comuns ausentes no dicionário PT→EN (`tabela`, `dinheiro`, `formatar`, `salvar`, `tempo`…), documentação em português vencendo código em perguntas sobre código, e packs que ignoravam a intenção da tarefa (decisões/testes pedidos explicitamente). Corrigi essas causas — não regras específicas por tarefa.
3. Como os ajustes foram guiados pelo conjunto principal, há **risco de sobreajuste**. Por isso o conjunto reservado foi escrito depois e nunca usado para ajustar: a busca híbrida ganhou da simples também ali (MRR 0,833 × 0,616).
4. Ajustes posteriores vieram de uso real da interface (não do conjunto): flexões em português (`validada` → `validar`), desempate de trechos e declarações pela cobertura de conceitos, limite de documentação genérica nos packs e testes procurados também para arquivos relacionados. As métricas permaneceram estáveis.

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
