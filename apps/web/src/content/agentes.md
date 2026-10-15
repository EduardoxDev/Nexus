# Conectar seu agente

A NEXUS conversa com os agentes de programação pelo protocolo MCP: o agente inicia o comando `nexus mcp serve` no seu computador e usa as ferramentas dele para buscar no código, montar Context Packs e ler a memória do projeto.

O jeito mais fácil é a tela **Integrações** do app: ela mostra a configuração de cada agente **já com os caminhos da sua instalação**, pronta para copiar. A NEXUS não altera os arquivos dos agentes; você cola onde cada um pede.

Nos exemplos abaixo, troque `CAMINHO_DO_NEXUS` pelo caminho da CLI (veja [Instalar](/docs/instalar/)) e `PASTA_DE_DADOS` pela pasta de dados da NEXUS (no Windows, `%LOCALAPPDATA%\Nexus`). O `--data-dir` faz o agente usar os mesmos projetos do app.

## Claude Code

Rode uma vez num terminal:

```bash
claude mcp add nexus -- CAMINHO_DO_NEXUS --data-dir PASTA_DE_DADOS mcp serve
```

O Claude Code inicia o servidor quando precisa e pede aprovação para as ferramentas no primeiro uso.

## Codex

Rode uma vez num terminal; o Codex grava a configuração em `~/.codex/config.toml` (confira com `codex mcp list`):

```bash
codex mcp add nexus -- CAMINHO_DO_NEXUS --data-dir PASTA_DE_DADOS mcp serve
```

## Cursor

Cole em `~/.cursor/mcp.json` (todos os projetos) ou em `.cursor/mcp.json` (só um projeto). Se o arquivo já tem servidores, junte a entrada `nexus` às existentes.

```json
{
  "mcpServers": {
    "nexus": {
      "type": "stdio",
      "command": "CAMINHO_DO_NEXUS",
      "args": ["--data-dir", "PASTA_DE_DADOS", "mcp", "serve"]
    }
  }
}
```

## VS Code

Cole em `.vscode/mcp.json` no projeto, ou use o comando **MCP: Open User Configuration** para valer em todos. Funciona no chat do Copilot em modo agente.

```json
{
  "servers": {
    "nexus": {
      "type": "stdio",
      "command": "CAMINHO_DO_NEXUS",
      "args": ["--data-dir", "PASTA_DE_DADOS", "mcp", "serve"]
    }
  }
}
```

## Claude Desktop

Em **Configurações → Desenvolvedor → Editar configuração**, cole no arquivo e reinicie o Claude Desktop. O arquivo fica em:

- Windows: `%APPDATA%\Claude\claude_desktop_config.json`
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Linux: `~/.config/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "nexus": {
      "command": "CAMINHO_DO_NEXUS",
      "args": ["--data-dir", "PASTA_DE_DADOS", "mcp", "serve"]
    }
  }
}
```

No Windows, dentro do JSON, cada `\` do caminho vira `\\`.

## O que o agente recebe

A NEXUS não envia nada para fora. O agente conectado, porém, manda ao provedor do modelo dele o que as ferramentas devolvem: trechos de código, caminhos e a memória do projeto. Conecte só agentes em que você confia para esses projetos. Mais em [Privacidade](/docs/privacidade/).

Testado de verdade: Claude Code. Os demais seguem o formato da documentação oficial de cada agente.
