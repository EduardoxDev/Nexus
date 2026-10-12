# ADR-0011 — Memória revisada pelo usuário e alertas de atualidade estáveis

- Estado: aceita
- Data: 2026-10-12

## Contexto

O NEXUS precisa guardar conhecimento que não está no código (decisões, convenções, lições) e avisar quando esse conhecimento — ou a documentação — deixa de corresponder ao repositório. Dois riscos guiam o desenho: (1) fontes automáticas (ADRs importados, agentes via MCP na Fase 5) sobrescreverem silenciosamente o que o usuário decidiu; (2) alertas ruidosos, que fazem a pessoa ignorar todos.

## Decisão

1. **Estados e quem revisa.** Memórias passam por `proposed → confirmed → archived | superseded` (e `archived → proposed`). Só quem revisa — o usuário pelo app ou pela CLI (`Actor::User`/`Actor::Cli`) — confirma, arquiva, restaura ou edita memórias confirmadas. Importação de ADRs e agentes (`AdrImport`/`Agent`) só criam e ajustam **propostas**; tentar mais que isso é `conflict`.
2. **Mudança em decisão confirmada vira proposta de substituição.** Se o ADR de uma decisão confirmada muda o texto da decisão, a importação cria uma nova proposta com `supersedes`; a confirmada só passa a `superseded` quando o usuário confirma a nova. Mudanças fora da seção de decisão só atualizam metadados (revisão do `system`, registrada).
3. **Histórico completo.** Cada mudança grava um retrato integral (tipo, estado, título, texto, referências, autor, nota). Edições usam controle otimista (`expected_revision`).
4. **Referências por hash, sem cópia de código** (ADR-0003). Uma referência guarda caminho, linhas e o hash do arquivo indexado; ao reabrir, o estado é `unchanged`, `changed`, `moved` (mesmo conteúdo em outro caminho), `not_indexed` ou `missing`.
5. **Context Packs só usam memórias confirmadas**, por referência (id + revisão). Ao reabrir, o pack mostra o texto atual e marca `revised`/`withdrawn`.
6. **Alertas com impressão digital.** Cada alerta tem `fingerprint` estável: o mesmo problema continua o mesmo alerta entre verificações; dispensado continua dispensado; o que deixa de ocorrer é resolvido (mantido 30 dias). A verificação completa roda depois de cada indexação que muda o índice e sob demanda; mudanças na memória reavaliam só os alertas de memória.
7. **Precisão antes de cobertura.** Blocos de código cercados, URLs, globs, placeholders e pastas de build são ignorados; código inline aceita caminho relativo a subprojeto (sufixo); identificadores só geram alerta se **existiam no código numa verificação anterior** e sumiram. Cada alerta diz se "requer revisão humana" quando o sistema não consegue decidir.

## Consequências

- Nomes citados que nunca existiram no código desde que o projeto foi adicionado não são cobrados (nas fixtures e no repositório do NEXUS eram APIs externas como `localStorage`); documentação já desatualizada antes da primeira indexação só é detectada para caminhos.
- Contradições são heurísticas (memórias confirmadas com o mesmo assunto e textos diferentes; decisão cuja origem mudou ou foi marcada como substituída) e sempre pedem revisão humana.
- Remover um projeto apaga sua memória (cascata); a interface avisa com a contagem antes de remover.
- A CLI conta como usuário. Agentes que chamarem a CLI herdam esse poder; a Fase 5 dá ao MCP o papel `agent`, limitado a propostas.
