# Bot do Discord da NEXUS — Etapa 1 (base + funções da NEXUS) — plano

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** um bot que responde, no Discord, com a busca e os Context Packs da NEXUS, sem nunca expor projetos da equipe fora dos canais da equipe.

**Architecture:** `apps/discord-bot` é um pacote TypeScript do monorepo executado direto pelo Node 24 (sem build). O bot inicia `nexus mcp serve` como processo filho e fala MCP (JSON-RPC por linha) com ele — o mesmo caminho do Claude Code. Uma camada de acesso decide, por canal, quais projetos podem ser consultados; os comandos só formatam o que o MCP devolve.

**Tech Stack:** Node 24 (TypeScript nativo por remoção de tipos), discord.js 14.27.0, Vitest 5, TypeScript ~6.0.3 (só checagem).

**Spec:** `docs/superpowers/specs/2026-10-12-discord-bot-design.md`

## Global Constraints

- Node ≥ 24; ESM; executar `.ts` direto com `node` (só sintaxe apagável: sem `enum`, `namespace` ou parâmetros de construtor com modificador).
- Dependência de produção única: `discord.js@14.27.0` (exata). Nada de dotenv: `process.loadEnvFile()`.
- Segredos só em `apps/discord-bot/.env` (no `.gitignore`); o bot nunca registra o token.
- Textos para o usuário em pt-BR; identificadores em inglês.
- Projeto `team` nunca é consultado, listado ou citado fora de `teamChannelIds`.
- Toda resposta com conteúdo cita `arquivo:início-fim`; sem resultados ⇒ diz que não encontrou.
- Toda mensagem enviada usa `allowedMentions: { parse: [] }`.
- Comandos registrados por servidor (`DISCORD_GUILD_ID`); intents só `Guilds` nesta etapa.

## Review Focus

1. **Menções vindas do código** (`@everyone`, `<@123>` dentro de um trecho) não podem notificar ninguém → `allowedMentions` vazio em toda resposta (Task 4, teste `replies never allow mentions`).
2. **Trecho com crases triplas** quebraria o bloco de código e mudaria a formatação → crases escapadas (Task 3, `escapes triple backticks in snippets`).
3. **Resposta acima de 2000 caracteres** é recusada pelo Discord → truncar com aviso; o pack inteiro vai como anexo `.md` (Task 3, `never exceeds 2000 chars`).
4. **Thread dentro de um canal da equipe** deve herdar o acesso do canal pai → `parentId` conta (Task 2, `threads inherit the parent channel`).
5. **`nexus mcp serve` morre ou trava** → a chamada falha em até 30 s com mensagem clara e o próximo comando reinicia o processo (Task 1, `restarts after the server exits` e `times out`).

---

### Task 1: Pacote e cliente MCP

**Files:**

- Create: `apps/discord-bot/package.json`, `tsconfig.json`, `vitest.config.ts`, `.env.example`, `.gitignore`
- Create: `apps/discord-bot/src/mcp/client.ts`
- Test: `apps/discord-bot/test/mcp.test.ts`

**Interfaces:**

- Produces:
  - `class McpClient { constructor(opts: { command: string; args: string[]; timeoutMs?: number }); callTool<T>(name: string, args: Record<string, unknown>): Promise<T>; close(): Promise<void> }` — `T` é o `structuredContent`.
  - `class McpToolError extends Error` (resultado com `isError: true`; `message` = texto devolvido) e `class McpUnavailableError extends Error` (processo morto, timeout ou JSON-RPC error).

- [ ] **Step 1: Pacote.** `package.json`: `"name": "@nexus/discord-bot"`, `"type": "module"`, `"private": true`, scripts `start: node src/index.ts`, `deploy: node src/deploy.ts`, `typecheck: tsc --noEmit`, `test: vitest run`; dependências: `discord.js` `14.27.0`; dev: `typescript ~6.0.3`, `vitest ^5.0.3`, `@types/node ^24.0.0` (mesmas do desktop). `tsconfig.json`: `module`/`moduleResolution` `nodenext`, `allowImportingTsExtensions`, `noEmit`, `strict`, `erasableSyntaxOnly`, `verbatimModuleSyntax`. `.env.example` com `DISCORD_TOKEN=`, `DISCORD_APP_ID=`, `DISCORD_GUILD_ID=`, `NEXUS_CLI=`, `NEXUS_DATA_DIR=`. `.gitignore`: `.env`, `bot.config.json`. Instalar com `corepack pnpm install` na raiz.
- [ ] **Step 2: Testes que falham** em `test/mcp.test.ts`, contra o binário real (`target/debug/nexus.exe`; `describe.skipIf` se ausente) com uma pasta de dados temporária onde a fixture `tests/fixtures/saas-app` foi adicionada e indexada pela CLI (`projects add <dir> --index`):
  - `lists the indexed project`: `callTool("list_projects", {})` → `projects[0].name === "saas-app"` e `indexedFiles > 0`.
  - `searches with sources`: `callTool("search_project", { query: "cálculo de frete", limit: 3 })` → `hits[0].relPath === "src/checkout/shipping.ts"`, `startLine >= 1`.
  - `tool errors become McpToolError`: `search_project` com `project: "nao-existe"` rejeita com `McpToolError`.
  - `restarts after the server exits`: matar o filho (`client["child"].kill()` via método de teste `__killForTests()`), a chamada seguinte funciona.
  - `times out`: `new McpClient({ command: process.execPath, args: ["-e", "setInterval(()=>{},1e9)"], timeoutMs: 300 })` → `callTool` rejeita com `McpUnavailableError` em < 1 s.
- [ ] **Step 3: Rodar** `corepack pnpm --filter @nexus/discord-bot test` → FAIL (módulo inexistente).
- [ ] **Step 4: Implementar `McpClient`.** Processo iniciado sob demanda; handshake legado: `initialize` com `protocolVersion: "2025-11-25"`, `clientInfo: { name: "nexus-discord-bot", version }`, depois `notifications/initialized`; leitura por linha do stdout; mapa `id → {resolve, reject, timer}`; ao sair o processo, rejeita pendentes com `McpUnavailableError` e zera o filho (a próxima chamada reinicia); stderr descartado (não registrar conteúdo).
- [ ] **Step 5: Rodar** os testes → PASS. `corepack pnpm --filter @nexus/discord-bot typecheck` → sem erros.
- [ ] **Step 6: Commit** `feat(discord-bot): pacote e cliente MCP sobre nexus mcp serve`.

### Task 2: Configuração e regra de acesso

**Files:**

- Create: `apps/discord-bot/src/config.ts`, `src/access.ts`, `bot.config.example.json`
- Test: `apps/discord-bot/test/access.test.ts`, `test/config.test.ts`

**Interfaces:**

- Produces:
  - `type Visibility = "public" | "team"`; `interface BotConfig { publicProject: string; teamProjects: string[]; teamChannelIds: string[] }`; `interface Env { token: string; appId: string; guildId: string; nexusCli: string; nexusDataDir: string }`.
  - `loadConfig(path: string): BotConfig` (lança `Error` em pt-BR com o campo inválido); `loadEnv(env: NodeJS.ProcessEnv): Env` (lança listando as variáveis faltantes, **sem** ecoar valores).
  - `interface ChannelRef { id: string; parentId: string | null }`; `isTeamChannel(cfg, ch: ChannelRef): boolean`; `visibleProjects(cfg, ch): string[]` (público primeiro); `canQuery(cfg, ch, project: string): boolean` (comparação sem diferenciar maiúsculas).

- [ ] **Step 1: Testes que falham** (`access.test.ts`), com `cfg = { publicProject: "nexus", teamProjects: ["otimiza"], teamChannelIds: ["100"] }`:
  - `public project anywhere`: `canQuery(cfg, {id:"999",parentId:null}, "nexus") === true`.
  - `team project only in team channels`: `canQuery(cfg, {id:"999",parentId:null}, "otimiza") === false`; com `id:"100"` → `true`.
  - `threads inherit the parent channel`: `{id:"555", parentId:"100"}` → `canQuery(..., "otimiza") === true`.
  - `unknown projects are never allowed`: `canQuery(cfg, {id:"100",parentId:null}, "outro") === false`.
  - `visible list hides team projects outside team channels`: `visibleProjects(cfg, {id:"999",parentId:null})` deep-equals `["nexus"]`; em `"100"` → `["nexus","otimiza"]`.
  - `config.test.ts`: `loadEnv({})` lança com mensagem contendo `DISCORD_TOKEN` e `DISCORD_GUILD_ID`; `loadEnv` com token preenchido nunca inclui o valor na mensagem de erro de outra variável faltante.
- [ ] **Step 2: Rodar** → FAIL.
- [ ] **Step 3: Implementar** `config.ts` e `access.ts`; `bot.config.example.json` com os três campos e IDs de exemplo.
- [ ] **Step 4: Rodar** → PASS.
- [ ] **Step 5: Commit** `feat(discord-bot): configuração e regra de acesso por canal (público/equipe)`.

### Task 3: Formatação para o Discord

**Files:**

- Create: `apps/discord-bot/src/format.ts`, `src/nexus/types.ts`
- Test: `apps/discord-bot/test/format.test.ts`

**Interfaces:**

- Consumes: formas do MCP (Task 1).
- Produces:
  - `types.ts`: `SearchHit { relPath; startLine; endLine; snippet; reasons: string[]; freshness: string; language: string | null; score: number }`, `SearchResult { hits: SearchHit[]; warnings: string[]; project: { id: number; name: string } }`, `ProjectStatus` (campos de `list_projects`), `PackResult { packId; summary; estimatedTokens; tokenBudget; export: string; sources: { relPath: string; startLine: number; endLine: number }[] }`.
  - `formatSearch(query: string, r: SearchResult, max = 3): string`; `formatPack(task: string, p: PackResult): { content: string; file: { name: string; data: Buffer } }`; `formatStatus(ps: ProjectStatus[]): string`; `escapeCode(s: string): string`. Constante `MAX_MESSAGE = 2000`.

- [ ] **Step 1: Testes que falham:**
  - `cites file and lines`: um hit `src/a.ts` 3–9 → saída contém `` `src/a.ts:3-9` `` e um bloco ` ```ts `.
  - `says when nothing was found`: `hits: []` → contém `Não encontrei nada sobre "x"`.
  - `escapes triple backticks in snippets`: snippet com ` ``` ` → saída não contém três crases dentro do bloco (`escapeCode` troca por `` `​`` ``).
  - `never exceeds 2000 chars`: 3 hits com snippets de 5000 caracteres → `length <= 2000` e termina com `…(trecho cortado)`.
  - `pack goes as attachment`: `formatPack` → `file.name === "context-pack.md"`, `file.data` é o `export`, `content` cita `~N de M tokens` e no máximo 10 fontes.
  - `marks stale results`: hit com `freshness: "modified"` → contém `(alterado desde a indexação)`.
- [ ] **Step 2: Rodar** → FAIL. **Step 3: Implementar.** **Step 4: Rodar** → PASS.
- [ ] **Step 5: Commit** `feat(discord-bot): formatação de busca, pack e estado para o Discord`.

### Task 4: Comando `/nexus` e inicialização

**Files:**

- Create: `apps/discord-bot/src/commands/nexus.ts`, `src/index.ts`, `src/deploy.ts`, `apps/discord-bot/README.md`
- Modify: `.github/workflows/ci.yml` (job frontend: `pnpm --filter @nexus/discord-bot typecheck` e `test`)
- Test: `apps/discord-bot/test/commands.test.ts`

**Interfaces:**

- Consumes: `McpClient` (Task 1), `BotConfig`/`canQuery`/`visibleProjects`/`ChannelRef` (Task 2), `format*` (Task 3).
- Produces:
  - `nexusCommand: SlashCommandBuilder` — subcomandos `pergunta <texto>`, `buscar <texto> [projeto]`, `pack <tarefa> [projeto]`, `status`; opção `projeto` com autocomplete.
  - `interface Tools { callTool<T>(name: string, args: Record<string, unknown>): Promise<T> }` (o `McpClient` satisfaz).
  - `handleNexus(i: NexusInteraction, deps: { tools: Tools; cfg: BotConfig }): Promise<void>` e `autocompleteProjects(focused: string, ch: ChannelRef, cfg: BotConfig): { name: string; value: string }[]`, onde `NexusInteraction` é o subconjunto usado de `ChatInputCommandInteraction` (`options`, `channelId`, `channel?.parentId`, `deferReply`, `editReply`, `reply`) — permite testar com objeto falso.

- [ ] **Step 1: Testes que falham** (objeto de interação falso que grava as respostas; `tools` falso que grava as chamadas):
  - `pergunta always uses the public project`: em canal comum → `search_project` chamado com `project: "nexus"`.
  - `team project is refused outside team channels`: `buscar` com `projeto: "otimiza"` no canal `"999"` → resposta efêmera `Este projeto não pode ser consultado neste canal.`, **nenhuma** chamada a `tools` e o texto não contém `otimiza`.
  - `buscar defaults to the team project in team channels`: canal `"100"`, sem `projeto` → `project: "otimiza"`.
  - `status lists only visible projects`: canal `"999"` com `list_projects` devolvendo `nexus` e `otimiza` → resposta não contém `otimiza`.
  - `replies never allow mentions`: toda chamada a `reply`/`editReply` tem `allowedMentions: { parse: [] }`.
  - `tool failures become friendly messages`: `tools` rejeita com `McpUnavailableError` → `editReply` com `A NEXUS não respondeu agora. Tente de novo em instantes.`
  - `autocomplete hides team projects`: `autocompleteProjects("", {id:"999",parentId:null}, cfg)` deep-equals `[{name:"nexus",value:"nexus"}]`.
- [ ] **Step 2: Rodar** → FAIL. **Step 3: Implementar** `nexus.ts` (respostas com `deferReply`; `pack` envia o anexo de `formatPack`). `index.ts`: `process.loadEnvFile()` se `.env` existir, `loadEnv`, `loadConfig("bot.config.json")`, `McpClient` com `[ "--data-dir", env.nexusDataDir, "mcp", "serve" ]`, `Client({ intents: [GatewayIntentBits.Guilds] })`, roteia `interactionCreate`; encerra o MCP em `SIGINT`/`SIGTERM`; log só de eventos (nunca token nem conteúdo). `deploy.ts`: `REST.put(Routes.applicationGuildCommands(appId, guildId), { body: [nexusCommand.toJSON()] })`.
- [ ] **Step 4: Rodar** testes → PASS; `typecheck` → sem erros.
- [ ] **Step 5: README** (pt-BR): criar o bot no Developer Portal, `.env`, `bot.config.json`, pasta de dados do bot com `nexus --data-dir <pasta> projects add <repo-da-nexus> --index`, `pnpm deploy`, `pnpm start`; seção "Segurança" com a regra público/equipe.
- [ ] **Step 6: Commit** `feat(discord-bot): comando /nexus (pergunta, buscar, pack, status) e inicialização`.

### Task 5: Verificação no Discord real

- [ ] **Step 1:** pasta de dados do bot com o repositório da NEXUS (projeto `nexus`, público) indexado.
- [ ] **Step 2:** o usuário preenche `.env` (token novo) e `bot.config.json`; `pnpm deploy` registra os comandos.
- [ ] **Step 3:** `pnpm start`; no servidor de teste: `/nexus pergunta como conecto no Claude Code?` cita `docs/mcp.md` com linhas; `/nexus status` mostra o projeto; `/nexus buscar` com projeto da equipe num canal comum é recusado.
- [ ] **Step 4:** registrar o resultado em `apps/discord-bot/README.md` e commit `docs(discord-bot): verificação da etapa 1`.
