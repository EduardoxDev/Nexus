# Servidor MCP

O NEXUS expõe seus projetos a agentes de programação (Claude Code, Codex, Cursor, VS Code, Claude Desktop e outros clientes MCP) pelo comando `nexus mcp serve`, que fala JSON-RPC 2.0 em stdio. O servidor usa o mesmo banco do app e da CLI; nada sai da máquina pelo NEXUS — mas veja [Privacidade](#privacidade).

## Conectar um agente

A tela **Integrações** do app mostra a configuração pronta de cada agente, com os caminhos desta instalação, para copiar. O app não grava nada nos arquivos dos agentes. Formatos conferidos na documentação oficial de cada um em 2026-10-13:

| Agente         | Como                                    | Onde                                                                                                                                  |
| -------------- | --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Claude Code    | `claude mcp add nexus -- <cli> <args>`  | terminal                                                                                                                              |
| Codex          | `codex mcp add nexus -- <cli> <args>`   | terminal (grava em `~/.codex/config.toml`)                                                                                            |
| Cursor         | JSON `mcpServers` com `"type": "stdio"` | `~/.cursor/mcp.json` ou `.cursor/mcp.json`                                                                                            |
| VS Code        | JSON `servers` com `"type": "stdio"`    | `.vscode/mcp.json` ou "MCP: Open User Configuration"                                                                                  |
| Claude Desktop | JSON `mcpServers`                       | `%APPDATA%\Claude\claude_desktop_config.json` (Windows), `~/Library/Application Support/Claude/` (macOS), `~/.config/Claude/` (Linux) |

Exemplo (Claude Code):

```bash
claude mcp add nexus -- "C:\caminho\nexus.exe" --data-dir "C:\Users\voce\AppData\Local\Nexus" mcp serve
```

Exemplo (Claude Desktop):

```json
{
  "mcpServers": {
    "nexus": {
      "command": "C:\\caminho\\nexus.exe",
      "args": [
        "--data-dir",
        "C:\\Users\\voce\\AppData\\Local\\Nexus",
        "mcp",
        "serve"
      ]
    }
  }
}
```

`--data-dir` garante que o servidor use o mesmo banco do app. Sem ele, vale `NEXUS_DATA_DIR` ou o diretório padrão (`%LOCALAPPDATA%\Nexus`). Na primeira vez, o agente pede aprovação para usar o servidor e as ferramentas.

O instalador traz a CLI ao lado do app, onde a tela Integrações a encontra. Em builds de desenvolvimento, compile com `cargo build -p nexus-cli --release` e use `target/release/nexus`.

Testado de verdade com o servidor: Claude Code (handshake e ferramentas). Os demais seguem o formato documentado de cada agente e usam o mesmo `initialize`; o servidor aceita as revisões listadas em [Protocolo](#protocolo).

## Protocolo

Servidor **dual-era** conforme a seção _Versioning and Compatibility_ da especificação [2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/basic/lifecycle) (verificada em 2026-10-12):

| Cliente                                           | Como é atendido                                                                                                                                                                                         |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Moderno (`2026-07-28`)                            | sem handshake; `_meta["io.modelcontextprotocol/protocolVersion"]` em cada requisição; `server/discover`; resultados com `resultType` e `serverInfo` em `_meta`; `tools/list` com `ttlMs` e `cacheScope` |
| Legado (`2025-11-25`, `2025-06-18`, `2025-03-26`) | `initialize` negocia a versão para o processo (versão desconhecida ⇒ `2025-11-25`); `notifications/initialized`; `ping`                                                                                 |

Erros: `-32700` JSON inválido, `-32600` requisição inválida (inclusive sem versão e sem `initialize`), `-32601` método desconhecido, `-32602` parâmetros inválidos ou ferramenta desconhecida, `-32022` versão não suportada (`data.supported`, `data.requested`). Erros de uso das ferramentas (projeto inexistente, argumento fora do limite, arquivo não indexado) voltam como resultado com `isError: true` e mensagem para o modelo corrigir.

Não implementados (não são necessários para ferramentas somente leitura): resources, prompts, `subscriptions/listen`, tasks, sampling, elicitation e o transporte HTTP. `listChanged` é `false`: a lista de ferramentas é fixa por versão do binário.

## Ferramentas

Todas recebem `project` (id, nome ou caminho), opcional quando há um único projeto. Cada uma tem `inputSchema` (sem propriedades extras) e `outputSchema`; o resultado traz `structuredContent` e o mesmo JSON em texto.

| Ferramenta              | Faz                                                                                                                                                                                             | Grava?                                    |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| `list_projects`         | projetos autorizados e estado do índice                                                                                                                                                         | não                                       |
| `get_project_overview`  | linguagens, tipos, pastas, manifestos, CI, mais alterados, memória e alertas                                                                                                                    | não                                       |
| `search_project`        | busca híbrida com motivos, trechos verificados e atualidade                                                                                                                                     | não                                       |
| `get_file_context`      | até 400 linhas de um arquivo **indexado**                                                                                                                                                       | não                                       |
| `get_file_history`      | últimos commits de um arquivo **indexado** (segue renomeações) e, com `startLine`/`endLine`, quem mudou aquelas linhas (`git blame`); `gitAvailable: false` sem git                             | não                                       |
| `get_symbol_context`    | declarações com tipo, contêiner e corpo (análise léxica de TS/JS, Python, Rust, Go, Java, C#, Kotlin, PHP, Ruby, C e C++; regex nas demais) e arquivos que importam a definição ou citam o nome | não                                       |
| `get_related_files`     | o que o arquivo importa e quem o importa (com a linha), testes que o importam ou têm nome correspondente, mesma pasta, manifesto, memórias que o referenciam                                    | não                                       |
| `get_project_memory`    | memória confirmada (propostas só com `includeProposals`)                                                                                                                                        | não                                       |
| `get_project_decisions` | decisões confirmadas, separadas de propostas e documentos de decisão não revisados                                                                                                              | não                                       |
| `build_context_pack`    | Context Pack para uma tarefa, exportado em Markdown ou JSON                                                                                                                                     | só o pack (referências) no banco do NEXUS |
| `get_stale_context`     | alertas de atualidade e contradições abertos                                                                                                                                                    | não                                       |
| `get_index_status`      | contagens, geração, job em execução e último job                                                                                                                                                | não                                       |
| `refresh_project_index` | atualização incremental + verificação de atualidade; 1 vez a cada 30 s por projeto, até 120 s por execução                                                                                      | índice do NEXUS                           |
| `get_context_sources`   | fontes de um pack, verificadas de novo contra o disco agora                                                                                                                                     | não                                       |

`nexus mcp tools` lista as ferramentas; `nexus mcp tools --json` imprime as definições completas.

## Segurança

- **Escopo**: só projetos autorizados; caminhos sempre relativos, resolvidos pelo core (recusa `..`, absolutos e qualquer arquivo não indexado — inclusive os sensíveis, que nunca são lidos).
- **Nenhuma ferramenta altera arquivos dos projetos.** As duas que gravam só tocam o banco do NEXUS.
- **Memória**: o MCP não confirma nem edita memória. Só o que o usuário confirmou aparece como decisão; propostas vêm separadas e rotuladas.
- **Conteúdo não confiável**: as instruções do servidor e o campo `notice` dizem que trechos e memória são dados, não instruções; o Markdown do pack usa cercas à prova de fechamento.
- **Auditoria**: cada chamada vira uma operação `mcp.tool` (ferramenta, resultado, código de erro, duração) e cada processo uma `mcp.session` (nome/versão do cliente). Os argumentos **não** são registrados. A tela Integrações mostra as chamadas ao vivo.
- **Limites**: argumentos validados (tamanho de consulta, limites numéricos, tipos, listas); linhas acima de 1 MiB recusadas; atualização do índice limitada por frequência e tempo.

## Privacidade

O NEXUS não envia dados para lugar nenhum. Um agente conectado, porém, envia o que as ferramentas devolvem (trechos de código, memória, caminhos) ao provedor do modelo que ele usa. Conecte apenas agentes em que você confia para esses projetos.

## Testes

- `crates/nexus-mcp/tests/session.rs`: handshake legado, requisições modernas sem estado, erros de protocolo, **todas as 14 ferramentas com o `structuredContent` validado contra o `outputSchema`**, erros de escopo e de entrada, limite de frequência e auditoria sem argumentos.
- `crates/nexus-cli/tests/cli.rs`: o binário `nexus mcp serve` por stdin/stdout, com stderr limpo.
- `apps/desktop/e2e/mcp.spec.ts`: sessão MCP real contra o mesmo banco do app aberto, com a chamada aparecendo em Integrações.
- Claude Code 2.1.296: `claude mcp list` com um `.mcp.json` de teste reconhece o servidor (fica "aguardando aprovação" até o usuário aprovar no `claude`). A conversa real com o modelo ainda não foi executada, porque envia o resultado das ferramentas à API do provedor.
