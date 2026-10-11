# Privacidade

## O que o NEXUS faz com seus dados

- **Nada sai da sua máquina.** O NEXUS não envia código, nomes de arquivos, consultas, memória técnica ou métricas para serviços externos.
- **Sem telemetria.** Não há coleta de uso, crash reports remotos ou "phone home".
- **A interface não acessa a internet.** A política de segurança de conteúdo (CSP) do app só permite o próprio app e o canal IPC local; o webview não tem acesso a plugins de rede.
- **Somente pastas autorizadas** (a partir da Fase 2): o NEXUS nunca varre o computador por conta própria.

## O que fica guardado localmente

| Item              | Onde                                          | Conteúdo                                                                                                 |
| ----------------- | --------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Banco             | `%LOCALAPPDATA%\Nexus\nexus.db`               | configurações, histórico de operações (metadados) e, a partir da Fase 2, índice dos projetos autorizados |
| Logs              | `%LOCALAPPDATA%\Nexus\logs\`                  | eventos estruturados (tipos, durações, códigos de erro, caminhos). **Nunca** conteúdo de arquivos        |
| Backups           | `%LOCALAPPDATA%\Nexus\backups\`               | cópias do banco feitas antes de migrações                                                                |
| Estado da janela  | `%APPDATA%\dev.nexus.desktop\`                | posição e tamanho da janela                                                                              |
| Cache do WebView2 | `%LOCALAPPDATA%\dev.nexus.desktop\EBWebView\` | dados internos do motor de renderização                                                                  |

## O que o índice de um projeto contém

- Caminhos relativos, tamanhos, datas de modificação, linguagem e tipo de cada arquivo.
- Hash (blake3) e intervalos de linhas do conteúdo indexado.
- O **índice invertido** da busca textual (FTS5 sem cópia do texto): os termos de cada bloco de até 60 linhas e suas posições. O texto não é armazenado, mas um índice invertido permite reconstruir trechos — trate o banco com o mesmo cuidado do código.
- Arquivos sensíveis (`.env`, chaves, credenciais, conteúdo com padrões de tokens/chaves) aparecem **apenas** com caminho, tamanho e motivo — sem hash e sem conteúdo.
- Ref e commit do HEAD do git (lidos de `.git`, sem executar `git`).

Remover o projeto do índice apaga tudo isso do banco; os arquivos da pasta nunca são alterados.

## Limites da proteção

- O banco **não é criptografado**. A proteção é a das permissões da sua conta no Windows: outros usuários sem privilégios de administrador não conseguem lê-lo, mas processos executados pela sua conta conseguem.
- Quem tem acesso administrativo à máquina tem acesso aos dados do NEXUS.

## Integrações futuras

Se integrações com serviços de IA em nuvem forem adicionadas, elas serão **opcionais e desligadas por padrão**, mostrarão exatamente quais dados serão enviados e permitirão cancelar antes do envio.

## Como apagar tudo

Feche o NEXUS e remova as pastas listadas acima. Desinstalar pelo Windows remove o programa; os dados em `%LOCALAPPDATA%\Nexus` são preservados para não apagar seu histórico sem confirmação.
