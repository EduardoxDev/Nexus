# Novidades

Cada versão da NEXUS, da mais nova para a mais antiga. A seção de cada versão vira a descrição da release no GitHub (`scripts/release-notes.mjs`).

## 0.3.0

### Destaques

- **Histórico do git.** Para cada arquivo, os últimos commits (seguindo renomeações) e, num trecho, quem mudou aquelas linhas. Aparece na prévia da busca, numa seção "Mudanças recentes" dos Context Packs e na ferramenta MCP nova `get_file_history`. Usa o `git` instalado, só para leitura e com os comandos do repositório desligados; sem `git`, o app avisa e segue sem histórico.
- **Mapa do código.** Uma tela nova mostra quem importa quem, com os arquivos mais centrais em destaque, filtro por pasta e modo agrupado. Clique num arquivo para abri-lo na Busca. A tabela "Mais centrais" traz os mesmos números para quem prefere ler.
- **Memória compartilhada pelo repositório.** Em Memória → "Exportar para o repositório", as memórias confirmadas viram arquivos em `.nexus/memory/`, com prévia antes de gravar. Quem clonar o projeto recebe essas memórias como **propostas** para revisar, nunca como decisões prontas.
- **Kotlin, PHP, Ruby, C e C++.** Agora são 12 linguagens: funções, classes, métodos e quem importa quem, inclusive pacotes Kotlin, PSR-4 do PHP, `require` do Ruby e `#include` com pastas `include/`.
- **Documentação no site.** Páginas de instalação, de como conectar cada agente, de privacidade e de novidades.

### Também

- A indexação espera alguns segundos quando o projeto está ocupado por um instante, em vez de recusar na hora.
- Bot do Discord: moderação (automod conservador e `/mod`) e gestão do servidor (boas-vindas, cargos por botão e tickets), tudo opcional e desligado até ser configurado.

### Antes de atualizar

- O banco local passa para a **versão 7** do schema (origem das memórias compartilhadas). Um backup é feito antes, automaticamente, na pasta `backups/` dos dados. Depois da atualização, a v0.2 não abre mais esse banco.
- Na primeira indexação depois de atualizar, os arquivos são analisados de novo uma vez (por causa das novas linguagens).
- O histórico do git precisa do `git` instalado no computador. Sem ele, todo o resto funciona normalmente.
- Os instaladores continuam **sem assinatura de código** do Windows e da Apple (as atualizações automáticas, essas sim, são conferidas por assinatura antes de instalar): veja como abrir em cada sistema na página [Instalar](https://eduardoxdev.github.io/Nexus/docs/instalar/).

## 0.2.0

### Destaques

- **Windows, Linux e macOS.** Pacotes para Windows (instalador), Linux (`.deb` e `.AppImage`) e macOS (Apple Silicon e Intel), todos com a CLI `nexus` incluída.
- **Atualização automática, com a sua permissão.** Na primeira abertura o app pergunta se pode verificar versões novas no GitHub. Sem resposta, não conecta em nada. Com permissão, verifica no máximo uma vez por dia; a instalação só acontece quando você clica, e a atualização é conferida por assinatura antes de instalar. Vale para o instalador do Windows, o `.AppImage` do Linux e o macOS; quem instalou pelo `.deb` atualiza baixando o `.deb` novo.
- **Mais agentes.** A tela Integrações mostra a configuração pronta para Claude Code, Codex, Cursor, VS Code e Claude Desktop, com os caminhos da sua instalação, para copiar.
- **Go, Java e C#.** Além de TypeScript, JavaScript, Python e Rust: funções, classes, métodos e quem importa quem. Em Go e C#, um import aponta para todos os arquivos do pacote ou namespace.

- **Visual novo.** O app ganhou o visual do site: grafite quase preto com azul só nos destaques, fonte Inter, cartões com um brilho que segue o mouse e um banner novo na Visão geral.

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
