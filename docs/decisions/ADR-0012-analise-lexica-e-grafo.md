# ADR-0012 — Análise léxica por linguagem e grafo de dependências com evidência

- Estado: aceita
- Data: 2026-10-12

## Contexto

A busca e os Context Packs dependiam de texto e heurísticas de nome; `get_symbol_context` e `get_related_files` (MCP) eram aproximações. O plano previa analisadores por linguagem "com tree-sitter apenas se a avaliação justificar" e um grafo de dependências incremental com evidência por aresta.

## Decisão

1. **Analisadores léxicos próprios** para TypeScript/TSX/JavaScript/JSX, Python e Rust, atrás do trait `Analyzer`. Um lexer mascara comentários e strings preservando offsets; as declarações saem linha a linha com uma pilha de blocos (chaves) ou de indentação (Python). A cobertura de cada um é declarada em código (`analysis::coverage()`) e aparece na interface.
2. **Tree-sitter não foi adotado**: nas medições (`docs/evaluation.md`), os analisadores léxicos acertaram 100% do gabarito conferido à mão (49 arquivos, 98 declarações, 88 imports com destino), tiveram revocação 727/727 contra uma busca independente no repositório do NEXUS, 39/39 numa amostra de declarações e 29/29 numa amostra de arestas (26/30 antes de corrigir `use super::*` em módulos de teste inline). O ganho não pagaria seis gramáticas nativas a mais no binário. Reavaliar se surgirem linguagens ou construções que a abordagem léxica não cubra.
3. **Análise por conteúdo, resolução por caminho.** Símbolos e imports são guardados por hash de conteúdo, junto com os chunks: arquivos idênticos compartilham a análise e uma renomeação não reanalisa nada. A resolução (import → arquivo) depende do conjunto de caminhos, então é refeita para o projeto inteiro, em memória, quando o índice muda de geração. Mede ~250 ms em 10 000 arquivos e 20 000 arestas.
4. **Evidência em toda aresta**: arquivo de origem, linha e especificador como escrito. O destino é um arquivo do projeto, um pacote externo ou "não resolvido" — nunca um palpite.
5. **Uso do grafo, validado contra a baseline da Fase 3** (sem ajuste no conjunto reservado): declarações da análise (tipo e linha nos motivos), centralidade com peso máximo 0,05 fixado antes de medir, e expansão 1-hop nos packs (testes que importam os arquivos principais e até 4 dependências diretas).
6. `INDEXER_VERSION` passou a 2, então índices antigos são reanalisados na próxima indexação, sem migração de dados.

## Consequências

- Construções fora da cobertura declarada (funções aninhadas, métodos de objetos literais, itens gerados por macros, imports montados em tempo de execução) não viram símbolos nem arestas.
- A reconstrução global do grafo a cada geração custa ~0,25 s em 10 000 arquivos e soma-se à verificação de atualidade (~0,32 s) em cada indexação com mudanças. Tornar a resolução incremental por arquivo fica como otimização futura.
- Para linguagens sem analisador (Go, Java…), continuam a regex de declarações e a citação textual do nome do módulo.
