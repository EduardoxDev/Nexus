# Bot do Discord da NEXUS — desenho

Data: 2026-10-12 · Estado: aprovado em conversa ("está ótimo, pode começar"); itens marcados
**[decisão do Claude]** são escolhas de implementação ainda abertas à revisão.

## Intenção

A NEXUS vai ser uma ferramenta pública. O servidor do Discord dela junta **a comunidade de
usuários e a equipe**. O bot serve a dois propósitos ao mesmo tempo:

1. **Algo legal e útil de verdade** para esse servidor: as funções da NEXUS pelo Discord,
   avisos de atualizações, moderação e gestão.
2. **Teste real da NEXUS** ("dogfooding"): o bot é construído com a ajuda da própria NEXUS, e
   ele mesmo usa a NEXUS do jeito que um agente externo usaria.

## O que o bot faz

### 1. Funções da NEXUS pelo Discord

- **Dúvidas da comunidade respondidas pela própria NEXUS.** O código e a documentação da
  NEXUS são abertos (MIT). O bot indexa o repositório da NEXUS e responde perguntas como
  "como conecto no Claude Code?" com a busca da NEXUS, **sempre citando as fontes**
  (arquivo e linhas), como o app faz. É suporte e demonstração do produto ao mesmo tempo.
- **Busca e Context Packs nos projetos da equipe**, só em canais da equipe (ver Regras).
- **Estado dos projetos**: índice, alertas de atualidade, decisões confirmadas na memória.

### 2. Atualizações

- **Para a comunidade**: anúncio de versões novas da NEXUS num canal de anúncios
  (publicado por alguém da equipe com um comando, a partir das notas de versão).
- **Para a equipe**: avisos automáticos num canal interno quando um projeto muda —
  indexação concluída ou com falha, alerta de documentação/memória desatualizada,
  contradição detectada, decisão nova registrada.

### 3. Moderação e gestão do servidor

- Moderação: filtro de spam/flood e links suspeitos, comandos de advertência, silenciar e
  banir, com registro de cada ação num canal de logs.
- Gestão: boas-vindas, cargos por botão (ex.: "usuário", "contribuidor"), tickets de suporte
  em canal privado.

## Regras (o que seria um fracasso mesmo funcionando)

- **Código privado em canal público.** Cada projeto indexado tem uma visibilidade:
  `public` (o repositório da NEXUS) ou `team` (projetos da equipe). Projetos `team` só podem
  ser consultados e citados em canais configurados como da equipe; fora deles o bot responde
  que não há acesso, sem revelar nomes de arquivos. Regra coberta por testes.
- **Inventar respostas.** Quando a NEXUS não encontra nada relevante, o bot diz que não
  encontrou — não "chuta". Toda resposta com conteúdo cita arquivo e linhas.
- **Seguir instruções contidas no código citado.** Trechos são dados, nunca comandos (mesma
  regra do MCP da NEXUS).
- **Vazar o token do bot ou segredos.** Token só em `.env` (fora do git); o bot nunca o
  registra. Arquivos sensíveis já são bloqueados pela NEXUS.
- **Moderação injusta e invisível.** Toda ação automática é registrada com o motivo e pode
  ser revertida por alguém da equipe.

## Como é feito

- **[decisão do Claude] TypeScript + discord.js 14**, em `apps/discord-bot` dentro do
  monorepo da NEXUS (mesmo CI, mesma base do Otimiza, que já usa discord.js 14 com ESM).
- **[decisão do Claude] O bot fala com a NEXUS pelo servidor MCP** (`nexus mcp serve`, stdio),
  exatamente como o Claude Code faz. Isso é o teste real do produto: o bot só tem as
  ferramentas que um agente tem (somente leitura, escopo nos projetos autorizados). O cliente
  MCP é pequeno e escrito no próprio bot (JSON-RPC por linha), sem dependência extra.
- **[decisão do Claude] Onde roda**: numa máquina com a CLI `nexus` e os projetos indexados
  (o PC da equipe ou um servidor). A pasta de dados do bot é separada da do app de cada pessoa
  (`--data-dir`), para controlar exatamente quais projetos ele enxerga.
- **[decisão do Claude] Estado do bot** (configuração de canais, advertências, tickets) em
  SQLite local do próprio bot — não no banco da NEXUS.
- **Token do Discord**: criado por alguém da equipe no Discord Developer Portal e colocado no
  `.env`; o Claude não manipula o token.
- **[decisão do Claude] Comandos de barra registrados por servidor** (`DISCORD_GUILD_ID`), que
  aparecem na hora; registro global só quando o bot for para outros servidores.
- **[decisão do Claude] Intents**: `Guilds`, `GuildMembers` (boas-vindas) e `GuildMessages` +
  `MessageContent` só a partir da etapa 3 (automod). As duas últimas são "privilegiadas" e
  precisam ser ligadas no Developer Portal.
- **[decisão do Claude] Testes com Vitest** (como o app desktop); o cliente MCP é testado
  contra o `nexus mcp serve` real sobre a fixture `saas-app`, e a regra de visibilidade com
  testes de unidade. Node 24, como no CI.

## Ordem de entrega [decisão do Claude]

1. **Base + funções da NEXUS**: cliente MCP, `/nexus pergunta` (comunidade, projeto público),
   `/nexus buscar`, `/nexus pack`, `/nexus status` (equipe), regra de visibilidade com testes.
2. **Atualizações**: avisos internos por consulta periódica ao MCP; anúncio de versão.
3. **Moderação**: automod (spam, flood, links), advertir/silenciar/banir, logs.
4. **Gestão**: boas-vindas, cargos por botão, tickets.

Cada etapa termina com o bot rodando num servidor de teste e testes automatizados verdes.

## Fora de escopo por ora

- Chat livre com IA no Discord (o bot cita fontes; não gera texto com modelo).
- Escrita na memória da NEXUS pelo Discord (o MCP não grava memória; decisões continuam
  confirmadas no app).
- Hospedagem gerenciada / painel web.

## Em aberto (não bloqueia a etapa 1)

- Nome e avatar do bot (padrão: "NEXUS" com o ícone do app).
- Regras exatas do automod (padrão conservador: só flood e links de convite de outros
  servidores).
