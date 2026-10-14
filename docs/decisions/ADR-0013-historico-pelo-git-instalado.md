# ADR-0013 — Histórico de arquivos pelo `git` instalado, só leitura

- Estado: aceita
- Data: 2026-10-14

## Contexto

Até a v0.2, a NEXUS só lia `.git/HEAD`, as refs e `packed-refs`, sem executar `git`. Saber quem mudou um arquivo, quando e por quê (o assunto do commit) ajuda o agente e a pessoa a entender o código. Para a v0.3, o usuário escolheu usar o `git` instalado em vez de ler o formato interno do repositório.

Ler objetos do git exigiria reimplementar zlib, packfiles, deltas e a detecção de renomeações, ou adicionar uma biblioteca nativa grande. O `git` já está em quase toda máquina de desenvolvimento e faz isso certo.

## Decisão

1. **Só `git log` e `git blame` (mais `ls-files` e `--version`)**, com argumentos fixos, sem shell e com o caminho depois de `--`. Sem `git` no computador, a função fica desligada: o app explica, o MCP devolve `gitAvailable: false` e o Context Pack sai sem a seção.
2. **Configuração do repositório não executa nada.** Um repositório clonado pode configurar comandos que o git roda sozinho. Toda chamada que consulta o repositório (`log`, `blame`, `ls-files`) usa `-c core.fsmonitor=false -c core.hooksPath=/dev/null --no-pager --literal-pathspecs`, e o `blame` usa `--no-textconv`. Também: `GIT_TERMINAL_PROMPT=0`, `GIT_OPTIONAL_LOCKS=0` (não grava no `.git`), stdin fechado e, no Windows, sem janela de console. Conferido na mão: sem essas opções, `git blame` roda o comando de `core.fsmonitor`; com elas, nada roda. Há teste automático para isso (`repository_config_cannot_run_commands`).
3. **Mesmo escopo da leitura de arquivos.** Só arquivos **indexados** do projeto: o caminho passa por `scan::check_project_path`, que recusa `..`, caminhos absolutos e links. Arquivos ignorados ou sensíveis, como `.env`, ficam de fora, inclusive o histórico deles.
4. **Limites**: 5 s por chamada (o processo é encerrado depois disso), no máximo 1 MiB de saída, até 200 commits por arquivo e `blame` de até 2 000 linhas.
5. **Nada é gravado.** O histórico é lido na hora (prévia da busca, ferramenta `get_file_history`, seção "Mudanças recentes" dos packs) e não vai para o banco. O e-mail do autor não é lido.
6. Fora de repositório, ou com o arquivo não versionado, a lista fica vazia, sem erro.

## Consequências

- O agente recebe nome do autor, data e assunto dos commits. Esses dados podem ser pessoais (nomes de colegas) e, como qualquer resultado de ferramenta, vão ao provedor do modelo que o agente usa.
- O assunto do commit é texto do repositório, então é tratado como dado, não instrução (vale o `notice` das ferramentas).
- Abrir ou criar um pack faz até 5 consultas ao git (os arquivos principais), com prazo total de 5 s. Um arquivo que sumiu é pulado; uma falha do git (prazo, erro) encerra as consultas e o pack sai com o que já tinha.
- `git log -z` com campos separados por NUL: nenhum texto do repositório desalinha os registros, e um registro cortado pelo limite de saída é descartado. `blame` com saída cortada é erro, não autoria parcial.
- O JSON exportado do pack ganhou `recentChanges` sem mudar `schemaVersion`: o campo é novo e opcional para quem lê, e `schemaVersion` versiona o plano gravado no banco, que não mudou.
- `get_file_context` não ganhou o último commit. Isso evita uma chamada ao git em toda leitura de arquivo; o agente usa `get_file_history` quando precisar.
- A busca não usa o histórico para ordenar os resultados. Fica para quando houver medição de que ajuda.
