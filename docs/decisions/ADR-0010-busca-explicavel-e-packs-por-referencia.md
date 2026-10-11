# ADR-0010 — Busca explicável e Context Packs por referência

- Estado: aceita
- Data: 2026-10-11

## Contexto

Agentes e pessoas precisam saber **por que** um arquivo foi escolhido e **se o trecho ainda é verdade**. Context Packs persistidos com cópias de código envelheceriam silenciosamente e duplicariam código no banco.

## Decisão

1. A pontuação híbrida é uma soma de componentes nomeados (conteúdo bm25, caminho, cobertura de termos, declaração, intenção por tipo, recência, citação direta). Cada componente aparece como **motivo** com seu peso; rótulos mostram a forma que realmente casou (`google→oauth`), nunca o termo original quando foi uma expansão.
2. A expansão PT→EN é um dicionário curado de termos técnicos mais tratamento leve de flexões — heurística documentada, sem modelo.
3. Context Packs guardam **apenas o plano**: fontes (caminho, papel, linhas, hash do arquivo na criação), análise da tarefa, riscos e limitações. O texto é lido do disco ao abrir/exportar e comparado com o hash: fontes alteradas aparecem como "modificado desde a criação do pack".
4. Prévia e trechos só leem arquivos com status `indexed` — arquivos ignorados (sensíveis, binários) nunca são lidos por esses caminhos.
5. A exportação Markdown cerca cada trecho com mais crases do que qualquer sequência do próprio conteúdo e abre com um aviso de que o conteúdo citado é dado, não instrução. O JSON tem `format` e `schemaVersion`.

## Consequências

- Um pack antigo pode perder trechos se arquivos forem apagados; isso aparece explicitamente (estado "ausente") em vez de mostrar conteúdo obsoleto.
- Ajustes de ranking precisam passar pela avaliação (`docs/evaluation.md`), com conjunto reservado para detectar sobreajuste.
