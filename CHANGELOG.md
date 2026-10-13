# Novidades

Cada versão da NEXUS, da mais nova para a mais antiga. A seção de cada versão vira a descrição da release no GitHub (`scripts/release-notes.mjs`).

## 0.2.0

### Destaques

- **Windows, Linux e macOS.** Pacotes para Windows (instalador), Linux (`.deb` e `.AppImage`) e macOS (Apple Silicon e Intel), todos com a CLI `nexus` incluída.
- **Atualização automática, com a sua permissão.** Na primeira abertura o app pergunta se pode verificar versões novas no GitHub. Sem resposta, não conecta em nada. Com permissão, verifica no máximo uma vez por dia; a instalação só acontece quando você clica, e a atualização é conferida por assinatura antes de instalar. Vale para o instalador do Windows, o `.AppImage` do Linux e o macOS; quem instalou pelo `.deb` atualiza baixando o `.deb` novo.
- **Mais agentes.** A tela Integrações mostra a configuração pronta para Claude Code, Codex, Cursor, VS Code e Claude Desktop, com os caminhos da sua instalação, para copiar.
- **Go, Java e C#.** Além de TypeScript, JavaScript, Python e Rust: funções, classes, métodos e quem importa quem. Em Go e C#, um import aponta para todos os arquivos do pacote ou namespace.

### Também

- Pastas do sistema no Linux e no macOS não podem ser adicionadas como projeto.
- A proteção da pasta de dados da NEXUS e das pastas do sistema compara o caminho real (`..`, links e nomes curtos do Windows não passam mais).
- O detalhe do projeto mostra "Não é um repositório git" inteiro, sem cortar.
- Bot do Discord: anúncio de versão nova, avisos da equipe e `/nexus versao`.

### Antes de atualizar

- O banco local passa para a **versão 6** do schema. Um backup é feito antes, automaticamente, na pasta `backups/` dos dados. Depois da atualização, a v0.1 não abre mais esse banco.
- Na primeira indexação depois de atualizar, os arquivos são analisados de novo uma vez (por causa das novas linguagens).
- Os pacotes **não são assinados**: no Windows o SmartScreen pode alertar ("Mais informações" → "Executar assim mesmo"); no macOS, veja `docs/release.md` para abrir o app.

## 0.1.0

Primeira versão: indexação local, busca com evidências, Context Packs, memória do projeto, alertas de atualidade, servidor MCP para o Claude Code e CLI `nexus` (Windows).
