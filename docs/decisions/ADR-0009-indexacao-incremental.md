# ADR-0009 — Indexação incremental por metadados, conteúdo endereçado e eventos como gatilho

- Estado: aceita
- Data: 2026-10-11

## Contexto

O índice precisa acompanhar o repositório sem reprocessar tudo, sobreviver a quedas no meio do trabalho, não travar o banco durante IO e não disputar CPU com o usuário. Eventos de sistema de arquivos são úteis, mas podem ser perdidos (buffers estourados, pastas montadas, alterações com o app fechado).

## Decisão

1. **Fast-path por metadados.** Um arquivo é considerado inalterado quando tamanho, mtime (ns) e versão do indexador coincidem com o registro. Só os demais são lidos.
2. **Conteúdo endereçado por hash (blake3).** Chunks e FTS pertencem ao conteúdo, não ao caminho: cópias idênticas e renomeações reaproveitam o índice. Renomeação = caminho removido cujo hash reaparece em caminho novo no mesmo job.
3. **Leitura estável.** Após ler, o arquivo é re-estatado; se tamanho/mtime mudaram, ele é tentado de novo ao fim do job e, persistindo, registrado como erro `unstable` (fica para a próxima passada) sem corromper o registro anterior.
4. **Lotes curtos como checkpoints.** Processamento paralelo (rayon, threads configuráveis; padrão metade dos núcleos) em lotes de 64 arquivos; cada lote é gravado numa transação curta. Nenhuma transação fica aberta durante IO de arquivo. Uma queda perde no máximo o lote em andamento.
5. **Trava por projeto no banco.** Índice único parcial sobre jobs ativos + heartbeat; jobs órfãos (sem heartbeat) viram `interrupted`. Funciona entre desktop, CLI e (futuramente) MCP.
6. **Eventos só disparam trabalho; a verdade é a varredura.** O monitor de arquivos (debounce de 1,5 s) apenas enfileira uma indexação incremental do projeto; a varredura completa com fast-path decide o que mudou. Isso respeita `.gitignore`/`.nexusignore` de forma idêntica em qualquer gatilho e corrige eventos perdidos. Reconciliação ao abrir o app e a cada 10 minutos.
7. **Um job por vez no desktop**, com pedidos do usuário à frente dos automáticos; pedidos repetidos para o mesmo projeto são combinados.

## Alternativas consideradas

- _Indexar apenas os caminhos dos eventos_: menos IO, mas exige reimplementar as regras de ignore por caminho e perde consistência quando eventos são descartados. Pode virar otimização futura se a varredura se mostrar cara — hoje a passada sem mudanças em 10 000 arquivos leva ~0,4 s (ver `docs/benchmarks.md`).
- _Hash de todos os arquivos a cada passada_: correto, mas lê o repositório inteiro; mantido como modo **reindexação completa** sob demanda.

## Consequências

- Alterações que preservam tamanho e mtime não são detectadas pelo modo incremental (raríssimo; coberto pela reindexação completa).
- O WAL recebe checkpoint passivo ao fim de cada job.
