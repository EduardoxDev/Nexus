# ADR-0003 — Não copiar o código indexado para o banco

- Estado: aceita (implementação na Fase 2/3)
- Data: 2026-10-10

## Contexto
Copiar todo o código para o SQLite duplica dados sensíveis e infla o banco. A busca precisa de índice invertido e de trechos com linhas exatas.

## Decisão
- Tabelas FTS5 **contentless** (`content=''`, `contentless_delete=1`) indexam o texto de cada chunk mais as subpalavras dos identificadores (`getUserName` → `get user name`).
- O banco guarda caminho, intervalo de linhas e hash blake3 de cada chunk/arquivo.
- Trechos exibidos são lidos do disco no momento da consulta; se o hash atual não bater com o indexado, o resultado é marcado como "modificado desde a indexação".
- Chunks são endereçados por conteúdo ⇒ arquivos idênticos são indexados uma vez.

## Consequências
Evidência sempre verificável e atualidade explícita; o custo é uma leitura de disco por resultado exibido (limitado ao top-N).
