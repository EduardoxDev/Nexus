# ADR-0006 — Recuperação sem ML como baseline

- Estado: aceita
- Data: 2026-10-10

## Decisão

A recuperação inicial combina FTS5 (bm25), correspondência de caminho e símbolo, intenção por tipo de arquivo, recência e relações estruturais, sempre com motivos explicáveis. Embeddings ou modelos locais só entram atrás de uma interface substituível **e** depois que a avaliação (Recall@k, Precision@k, MRR) mostrar ganho real sobre essa baseline. Nenhum banco vetorial externo.
