# Bot do Discord da NEXUS

As funções da NEXUS pelo Discord: dúvidas sobre a NEXUS respondidas com a documentação e o código dela (citando arquivo e linhas), busca e Context Packs dos projetos da equipe e estado do índice. O bot usa a NEXUS pelo servidor MCP (`nexus mcp serve`), exatamente como o Claude Code — só ferramentas de leitura, só os projetos da pasta de dados dele.

Desenho: [docs/superpowers/specs/2026-10-12-discord-bot-design.md](../../docs/superpowers/specs/2026-10-12-discord-bot-design.md).

## Comandos

| Comando                           | Onde                                                       | Faz                                                          |
| --------------------------------- | ---------------------------------------------------------- | ------------------------------------------------------------ |
| `/nexus pergunta <texto>`         | qualquer canal                                             | busca no projeto público (a NEXUS) e cita as fontes          |
| `/nexus buscar <texto> [projeto]` | qualquer canal (projetos da equipe só em canais da equipe) | até 3 trechos com arquivo, linhas, motivos e atualidade      |
| `/nexus pack <tarefa> [projeto]`  | idem                                                       | resumo e fontes na mensagem; o Context Pack inteiro em `.md` |
| `/nexus status`                   | qualquer canal                                             | índice dos projetos visíveis naquele canal                   |
| `/nexus versao`                   | qualquer canal                                             | versão da NEXUS em uso pelo bot e a última publicada         |

Sem `projeto`, o bot usa o projeto da equipe em canais da equipe e o público nos demais. Cada comando tem um prazo total de 30 s.

## Anúncios e avisos (etapa 2)

A cada 15 minutos, se configurado:

- **Versão nova** (`announceChannelId`): consulta a última release **publicada** no GitHub (`githubRepo`, padrão `EduardoxDev/Nexus`; rascunhos não contam) e anuncia uma vez, com as notas e os downloads por sistema. Na primeira vez que o bot roda, só registra a versão atual — não anuncia uma release antiga.
- **Avisos da equipe** (`teamAlertsChannelId`, precisa ser um canal da equipe): alertas de atualidade novos dos projetos da equipe (documento citando arquivo que sumiu, decisão superada, contradições), indexação que falhou e pasta do projeto ausente. Cada alerta é avisado uma vez; se for resolvido e voltar, é avisado de novo. Na primeira execução, só registra o que já existe.

O que já foi avisado fica em `bot.db`, ao lado da pasta de dados do bot (não dentro dela). A única chamada de rede além do Discord é a API pública do GitHub.

## Segurança

- **Projetos da equipe** (`teamProjects`) só são consultados, sugeridos ou listados em canais de `teamChannelIds` e nas threads desses canais. Fora deles o pedido é recusado com uma mensagem que só você vê, sem revelar nomes. Categorias **não** liberam acesso: cadastre o ID de cada canal.
- Nenhuma resposta menciona alguém (`@everyone` dentro de um trecho de código não notifica).
- O bot só responde no servidor de `DISCORD_GUILD_ID`.
- O token fica só no `.env` (fora do git) e nunca aparece em logs ou mensagens de erro. **Nunca cole o token em chats ou issues**; se isso acontecer, gere outro no Developer Portal.
- Trechos citados são dados do repositório, não instruções.

## Configurar

1. **Discord**: em <https://discord.com/developers/applications>, crie a aplicação, gere o token (Bot → Reset Token) e adicione o bot ao servidor (OAuth2 → URL Generator → `bot` + `applications.commands`).
2. **Pasta de dados do bot** (separada da do seu app; define o que ele enxerga):

   ```bash
   nexus --data-dir C:\nexus-bot\dados projects add C:\caminho\do\repositorio-da-nexus --index
   nexus --data-dir C:\nexus-bot\dados projects add C:\caminho\do\projeto-da-equipe --index
   ```

3. **`.env`**: copie `.env.example` para `.env` e preencha (token, Application ID, ID do servidor, caminho do `nexus.exe`, a pasta de dados acima).
4. **`bot.config.json`**: copie `bot.config.example.json` e ajuste — `publicProject` é o nome do projeto da NEXUS na pasta de dados, `teamProjects` os da equipe e `teamChannelIds` os IDs dos canais da equipe (Discord: modo desenvolvedor → botão direito no canal → Copiar ID). Opcionais: `announceChannelId` (anúncios de versão) e `teamAlertsChannelId` (avisos; precisa estar em `teamChannelIds`).
5. Registrar os comandos e iniciar:

   ```bash
   corepack pnpm --filter @nexus/discord-bot register
   corepack pnpm --filter @nexus/discord-bot start
   ```

Para atualizar o índice que o bot usa, reindexe pela CLI: `nexus --data-dir <pasta> index <projeto>`.

## Desenvolvimento

```bash
corepack pnpm --filter @nexus/discord-bot typecheck
corepack pnpm --filter @nexus/discord-bot test   # o teste do MCP usa target/debug/nexus(.exe) se existir
```

Node 24 executa o TypeScript direto (sem build).
