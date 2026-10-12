/**
 * Cliente MCP mínimo para `nexus mcp serve` (stdio, uma mensagem JSON-RPC por linha).
 *
 * O bot usa a NEXUS exatamente como um agente usa: só as ferramentas do servidor MCP. O processo
 * é iniciado sob demanda e reiniciado na próxima chamada se terminar; cada chamada tem prazo.
 */
import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { createInterface } from "node:readline";

const PROTOCOL_VERSION = "2025-11-25";
const CLIENT_INFO = { name: "nexus-discord-bot", version: "0.1.0" };

/** A ferramenta respondeu com erro de uso (`isError: true`); `message` é o texto da NEXUS. */
export class McpToolError extends Error {
  override name = "McpToolError";
}

/** O servidor não respondeu: processo encerrado, prazo esgotado ou erro de protocolo. */
export class McpUnavailableError extends Error {
  override name = "McpUnavailableError";
}

interface Pending {
  /** Processo que recebeu a requisição: só a queda dele a rejeita. */
  child: ChildProcessWithoutNullStreams;
  resolve: (value: unknown) => void;
  reject: (error: Error) => void;
  timer: NodeJS.Timeout;
}

interface RpcResponse {
  id?: number;
  result?: unknown;
  error?: { code: number; message: string };
}

interface ToolResult {
  isError?: boolean;
  structuredContent?: unknown;
  content?: { type: string; text?: string }[];
}

export interface McpClientOptions {
  command: string;
  args: string[];
  /** Prazo por requisição (padrão 30 s). */
  timeoutMs?: number;
}

export class McpClient {
  private readonly opts: Required<McpClientOptions>;
  private child: ChildProcessWithoutNullStreams | null = null;
  private ready: Promise<void> | null = null;
  /** Saída do último processo descartado: o próximo só inicia depois (o banco fica travado). */
  private lastExit: Promise<void> = Promise.resolve();
  private nextId = 1;
  private readonly pending = new Map<number, Pending>();

  constructor(opts: McpClientOptions) {
    this.opts = { timeoutMs: 30_000, ...opts };
  }

  /** Chama uma ferramenta e devolve o `structuredContent`. */
  async callTool<T>(name: string, args: Record<string, unknown>): Promise<T> {
    await this.start();
    const result = (await this.request("tools/call", {
      name,
      arguments: args,
    })) as ToolResult;
    if (result.isError) {
      const text = result.content?.find((c) => c.type === "text")?.text ?? "erro da ferramenta";
      throw new McpToolError(text);
    }
    return result.structuredContent as T;
  }

  async close(): Promise<void> {
    const child = this.child;
    this.child = null;
    this.ready = null;
    if (!child || child.exitCode !== null) return;
    // Resolve só depois que o processo sai: no Windows o banco fica travado enquanto ele vive.
    const exited = new Promise<void>((resolve) => child.once("exit", () => resolve()));
    child.stdin.end();
    child.kill();
    await exited;
  }

  /** Só para testes: simula a queda do servidor. */
  __killForTests(): void {
    this.child?.kill();
  }

  /** Encerra um processo e guarda a promessa da saída dele para o próximo início esperar. */
  private discard(child: ChildProcessWithoutNullStreams): void {
    if (this.child === child) {
      this.child = null;
      this.ready = null;
    }
    if (child.exitCode === null && child.signalCode === null) {
      this.lastExit = new Promise((resolve) => {
        child.once("exit", () => resolve());
        child.once("error", () => resolve());
      });
      child.kill();
    }
  }

  private start(): Promise<void> {
    // Um processo morto ou marcado para morrer conta como ausente, mesmo antes do evento `exit`.
    const alive = this.child && !this.child.killed && this.child.exitCode === null;
    if (alive && this.ready) return this.ready;
    if (this.child) this.discard(this.child);
    const previous = this.lastExit;
    this.ready = previous.then(() => this.spawnAndInit());
    const ready = this.ready;
    ready.catch(() => {
      if (this.ready === ready) this.ready = null;
    });
    return ready;
  }

  private spawnAndInit(): Promise<void> {
    // Segredos do bot (token do Discord etc.) não vão para o processo da NEXUS.
    const env = Object.fromEntries(
      Object.entries(process.env).filter(([key]) => !key.startsWith("DISCORD_")),
    );
    const child = spawn(this.opts.command, this.opts.args, {
      stdio: ["pipe", "pipe", "pipe"],
      env,
    });
    this.child = child;
    // O stderr do servidor tem logs de diagnóstico; o bot não os repassa.
    child.stderr.resume();
    createInterface({ input: child.stdout }).on("line", (line) => this.onLine(line));
    const onGone = () => {
      if (this.child === child) {
        this.child = null;
        this.ready = null;
      }
      this.rejectAll(child, new McpUnavailableError("o servidor MCP da NEXUS foi encerrado"));
    };
    child.on("exit", onGone);
    child.on("error", onGone);

    const init = this.request("initialize", {
      protocolVersion: PROTOCOL_VERSION,
      capabilities: {},
      clientInfo: CLIENT_INFO,
    }).then(() => this.notify("notifications/initialized"));
    // Uma falha na inicialização não deixa o cliente preso: a próxima chamada tenta de novo.
    init.catch(() => this.discard(child));
    return init;
  }

  private request(method: string, params: unknown): Promise<unknown> {
    const child = this.child;
    if (!child) return Promise.reject(new McpUnavailableError("servidor MCP não iniciado"));
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new McpUnavailableError(`a NEXUS não respondeu em ${this.opts.timeoutMs} ms`));
        // Um servidor travado é descartado; a próxima chamada inicia outro depois que ele sair.
        this.discard(child);
      }, this.opts.timeoutMs);
      this.pending.set(id, { child, resolve, reject, timer });
      child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id, method, params })}\n`);
    });
  }

  private notify(method: string): void {
    this.child?.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", method })}\n`);
  }

  private onLine(line: string): void {
    let msg: RpcResponse;
    try {
      msg = JSON.parse(line) as RpcResponse;
    } catch {
      return;
    }
    if (typeof msg.id !== "number") return;
    const pending = this.pending.get(msg.id);
    if (!pending) return;
    this.pending.delete(msg.id);
    clearTimeout(pending.timer);
    if (msg.error)
      pending.reject(new McpUnavailableError(`erro MCP ${msg.error.code}: ${msg.error.message}`));
    else pending.resolve(msg.result);
  }

  private rejectAll(child: ChildProcessWithoutNullStreams, error: Error): void {
    for (const [id, p] of this.pending) {
      if (p.child !== child) continue;
      clearTimeout(p.timer);
      p.reject(error);
      this.pending.delete(id);
    }
  }
}
