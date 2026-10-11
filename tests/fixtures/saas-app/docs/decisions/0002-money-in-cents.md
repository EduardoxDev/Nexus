# ADR 0002 — Dinheiro em centavos inteiros

Estado: aceita

## Contexto
Somas com ponto flutuante geraram diferenças de um centavo em faturas.

## Decisão
Todo valor monetário é um inteiro em centavos. Conversões para exibição acontecem apenas em `formatCents`.
