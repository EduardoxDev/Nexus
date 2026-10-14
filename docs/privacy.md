# Privacidade

## O que o NEXUS faz com seus dados

- **Nada sai da sua máquina.** O NEXUS não envia código, nomes de arquivos, consultas, memória técnica ou métricas para serviços externos.
- **Sem telemetria.** Não há coleta de uso, crash reports remotos ou "phone home".
- **Atualizações só com permissão (v0.2).** A única conexão de rede do app é a consulta ao GitHub por versões novas. Na primeira abertura o app pergunta; sem resposta, não consulta nada. Com permissão, consulta ao abrir, no máximo uma vez por dia, e envia ao GitHub apenas o que qualquer download envia (seu IP e a versão do app no pedido) — nada dos projetos. Dá para desligar em Configurações, e "Verificar agora" funciona mesmo desligado. Cada consulta fica registrada em Diagnóstico (`updates.check`). A consulta é feita pelo Rust; o webview continua sem acesso à rede.
- **A interface não acessa a internet.** A política de segurança de conteúdo (CSP) do app só permite o próprio app e o canal IPC local; o webview não tem acesso a plugins de rede.
- **Somente pastas autorizadas** (a partir da Fase 2): o NEXUS nunca varre o computador por conta própria.

## O que fica guardado localmente

| Item              | Onde                                          | Conteúdo                                                                                                |
| ----------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Banco             | `%LOCALAPPDATA%\Nexus\nexus.db`               | configurações, histórico de operações (metadados), índice dos projetos autorizados e memória do projeto |
| Logs              | `%LOCALAPPDATA%\Nexus\logs\`                  | eventos estruturados (tipos, durações, códigos de erro, caminhos). **Nunca** conteúdo de arquivos       |
| Backups           | `%LOCALAPPDATA%\Nexus\backups\`               | cópias do banco feitas antes de migrações                                                               |
| Estado da janela  | `%APPDATA%\dev.nexus.desktop\`                | posição e tamanho da janela                                                                             |
| Cache do WebView2 | `%LOCALAPPDATA%\dev.nexus.desktop\EBWebView\` | dados internos do motor de renderização                                                                 |

## O que o índice de um projeto contém

- Caminhos relativos, tamanhos, datas de modificação, linguagem e tipo de cada arquivo.
- Hash (blake3) e intervalos de linhas do conteúdo indexado.
- O **índice invertido** da busca textual (FTS5 sem cópia do texto): os termos de cada bloco de até 60 linhas e suas posições. O texto não é armazenado, mas um índice invertido permite reconstruir trechos — trate o banco com o mesmo cuidado do código.
- Arquivos sensíveis (`.env`, chaves, credenciais, conteúdo com padrões de tokens/chaves) aparecem **apenas** com caminho, tamanho e motivo — sem hash e sem conteúdo.
- Ref e commit do HEAD do git (lidos de `.git`, sem executar `git`).

## Histórico do git

Para mostrar quem mudou um arquivo e quando, a NEXUS executa o `git` instalado no computador (só `log`, `blame`, `ls-files` e `--version`), nunca um comando que grave no repositório. Comandos configurados pelo próprio repositório (`core.fsmonitor`, hooks, `textconv`) ficam desligados. Sem `git` instalado, nada é executado: o app mostra um aviso ("Instale o git…") e a ferramenta `get_file_history` responde `gitAvailable: false`, sem commits.

- Só para arquivos **indexados**: arquivos que o índice marcou como ignorados ou sensíveis não têm o histórico consultado. Um arquivo que ganhou um segredo depois da última indexação só passa a ser tratado como sensível na próxima indexação; até lá, o histórico dele (nunca o conteúdo) pode ser consultado.
- Lidos: hash, nome do autor, data e assunto dos commits. O e-mail do autor não é lido.
- Nada disso é gravado no banco: o histórico é lido na hora em que aparece.
- Um agente conectado que use `get_file_history` ou um Context Pack recebe esses dados (inclusive nomes de pessoas) e os envia ao provedor do modelo, como qualquer resultado de ferramenta. Detalhes em `docs/decisions/ADR-0013-historico-pelo-git-instalado.md`.

Remover o projeto do índice apaga tudo isso do banco; os arquivos da pasta nunca são alterados.

## Memória compartilhada (`.nexus/memory/`)

O app e o servidor MCP só gravam dentro de um projeto em um caso: "Exportar para o repositório", na tela Memória, e só com o seu clique, depois de uma prévia do que será criado ou alterado. (Na linha de comando, `nexus context --output ARQUIVO` grava o Context Pack no caminho que você escolher.)

- Só memórias **confirmadas** são exportadas, uma por arquivo em `.nexus/memory/`: título, texto, tipo, revisão, datas e referências (caminho, linhas e hash). As referências não copiam trechos dos arquivos. O texto da memória vai inteiro: o que você escreveu (inclusive código que tenha colocado nele) e, numa memória vinda de um ADR importado, a seção de decisão copiada do documento.
- A NEXUS não apaga nenhum arquivo dessa pasta e não grava fora dela (links e junctions são recusados).
- O que estiver no repositório passa a ser visto por quem tiver acesso a ele, como qualquer arquivo commitado. Confira a prévia antes de exportar.
- Na leitura, arquivos de `.nexus/memory/` viram **propostas** no seu banco, nunca memórias confirmadas. Detalhes em `docs/decisions/ADR-0014-memoria-compartilhada-pelo-repositorio.md`.

## Memória do projeto

- A memória guarda **texto**: o que você escreve e, ao importar ADRs, a seção de decisão do documento (até 4 000 caracteres por ADR). Cada revisão guarda o texto completo daquela versão.
- Referências a arquivos guardam caminho, linhas e hash — não o código.
- O histórico de operações registra apenas IDs, tipo, estado e revisão das memórias; nunca título ou texto.
- Alertas de atualidade guardam caminhos, linhas e o que foi citado (um caminho ou nome), não trechos de código.
- Diferente do índice, **a memória não é reconstruível** a partir dos arquivos: remover o projeto a apaga. Faça backup do banco se ela for importante.

## Limites da proteção

- O banco **não é criptografado**. A proteção é a das permissões da sua conta no Windows: outros usuários sem privilégios de administrador não conseguem lê-lo, mas processos executados pela sua conta conseguem.
- Quem tem acesso administrativo à máquina tem acesso aos dados do NEXUS.

## Agentes conectados (MCP)

O servidor MCP (`nexus mcp serve`) roda localmente e responde apenas ao cliente que o iniciou. O NEXUS continua sem enviar nada para fora, **mas o agente conectado envia ao provedor do modelo dele o que as ferramentas devolvem**: trechos de código, caminhos, memória e alertas. Conecte apenas agentes em que você confia para esses projetos. O histórico local registra cada chamada (ferramenta, resultado, duração), nunca os argumentos.

## Integrações futuras

Se integrações diretas do NEXUS com serviços de IA em nuvem forem adicionadas, elas serão **opcionais e desligadas por padrão**, mostrarão exatamente quais dados serão enviados e permitirão cancelar antes do envio.

## Como apagar tudo

Feche o NEXUS e remova as pastas listadas acima. Desinstalar pelo Windows remove o programa; os dados em `%LOCALAPPDATA%\Nexus` são preservados para não apagar seu histórico sem confirmação.
