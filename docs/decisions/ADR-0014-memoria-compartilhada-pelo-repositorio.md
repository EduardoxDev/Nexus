# ADR-0014 — Memória compartilhada pelo repositório (`.nexus/memory/`)

- Estado: aceita
- Data: 2026-10-14

## Contexto

A memória do projeto vive no banco local de cada pessoa. Uma equipe que trabalha no mesmo repositório não tinha como compartilhar decisões e convenções confirmadas, a não ser escrevendo ADRs à mão. Para a v0.3, o usuário escolheu compartilhar pelo próprio repositório, com duas condições:

- a NEXUS só grava no repositório com um clique explícito, depois de mostrar uma prévia;
- o que chega de outra pessoa entra como **proposta**, nunca como memória confirmada.

## Decisão

1. **Formato**: um arquivo Markdown por memória em `.nexus/memory/<título>-<fim do id>.md`, com front-matter.
   - Campos:
     - `format: nexus-memory/1`, `id` (o mesmo da memória, estável entre bancos);
     - `kind`, `title`, `revision`, `createdAt`, `updatedAt` (RFC 3339);
     - `supersedes`, quando a memória substitui outra;
     - `refs`, com `relPath`, `startLine`, `endLine` e `contentHash`.
   - O texto da memória vem depois do front-matter.
   - Textos ficam entre aspas, como JSON, para não depender de um leitor YAML completo.
   - O `status` não vai no arquivo: quem recebe sempre decide.
   - É legível no GitHub e revisável num PR.
2. **Exportar** (app, Memória → "Exportar para o repositório"):
   - só memórias **confirmadas**;
   - primeiro uma prévia (criar, atualizar ou sem mudança, mais os arquivos da pasta que não correspondem a nada);
   - a gravação leva o `token` da prévia e é recusada se algo mudou desde então;
   - grava só dentro de `.nexus/memory/`, conferindo que nenhum trecho do caminho é link ou junction e que o caminho real fica dentro do projeto;
   - escreve num arquivo temporário e troca pelo definitivo;
   - **nunca apaga** arquivos. Os que sobram aparecem na prévia, e a pessoa decide.
   - É a única escrita da NEXUS dentro de um projeto.
3. **Ler** (a cada indexação, se a pasta existir): cada arquivo vira, no máximo, uma **proposta**. A memória é reconhecida pelo `id`:
   - **id desconhecido**: proposta nova com o mesmo `id` e a mesma revisão do arquivo. Se `supersedes` apontar para uma memória confirmada aqui, a proposta a substitui quando a pessoa confirmar;
   - **id de memória confirmada**:
     - texto igual: nada muda;
     - revisão menor que a daqui: o arquivo está atrasado e é ignorado;
     - senão: **proposta de revisão** (`supersedes` = a confirmada). Ela acompanha o arquivo enquanto não for revisada;
   - **id de proposta lida de arquivo**: acompanha o arquivo;
   - **id de memória arquivada ou substituída**: ignorado;
   - **arquivo de uma memória que outro arquivo da pasta substitui**: ignorado.
   - Arquivos inválidos são listados com o motivo, sem derrubar os outros: formato desconhecido, campos inválidos, caminho com `..`, link, mais de 64 KiB ou texto que não é UTF-8.
4. **Banco**:
   - nova origem `shared_file` e novo autor de revisão `shared_file` (migration v7);
   - o SQLite não altera um CHECK no lugar, então `memories` e `memory_revisions` são recriadas com os mesmos dados;
   - o migrador desliga as chaves estrangeiras durante as migrations (senão o `DROP` apagaria em cascata) e confere `foreign_key_check` antes de cada commit.

## Consequências

- **Um arquivo malicioso vindo de um PR não muda nada sozinho.** Ele vira proposta e só passa a valer se a pessoa confirmar. A memória confirmada continua igual até lá.
- Edições simultâneas da mesma memória em dois computadores aparecem como proposta de revisão nos dois lados, ou como conflito no próprio git.
- **Identidade**: a memória lida de outro banco começa na revisão do arquivo. Assim, uma edição feita lá (revisão maior) é reconhecida como nova, e uma cópia velha (menor) é ignorada.
- **Arquivos obsoletos**: depois que uma memória é substituída, o arquivo antigo continua na pasta até alguém apagá-lo. Num banco novo, ele é ignorado se outro arquivo da pasta o substitui.
- **Ids repetidos**: um id que já pertence a outro projeto do mesmo banco é recusado.
- A importação de ADRs ignora `.nexus/`: a pasta tem leitura própria.
