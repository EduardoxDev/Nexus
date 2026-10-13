/**
 * Uma instância por pasta: dois bots com o mesmo token disputam cada comando (um responde, o
 * outro recebe "interação desconhecida") e cada um mantém a própria conexão com a NEXUS.
 */
import { openSync, readFileSync, rmSync, writeSync, closeSync } from "node:fs";

export class AlreadyRunningError extends Error {
  override name = "AlreadyRunningError";
}

function isAlive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (e) {
    // EPERM: existe, mas é de outro usuário.
    return (e as NodeJS.ErrnoException).code === "EPERM";
  }
}

/** Cria o arquivo de trava com o PID; recusa se outro processo vivo já o tem. */
export function acquireInstanceLock(path: string): { release(): void } {
  for (let attempt = 0; attempt < 2; attempt++) {
    try {
      const fd = openSync(path, "wx");
      writeSync(fd, String(process.pid));
      closeSync(fd);
      return {
        release: () => rmSync(path, { force: true }),
      };
    } catch (e) {
      if ((e as NodeJS.ErrnoException).code !== "EEXIST") throw e;
      const pid = Number.parseInt(readFileSync(path, "utf8").trim(), 10);
      if (Number.isInteger(pid) && pid > 0 && isAlive(pid)) {
        throw new AlreadyRunningError(
          `o bot já está rodando (processo ${pid}). Feche a outra janela antes de iniciar de novo.`,
        );
      }
      // Trava de um processo que morreu: descarta e tenta de novo.
      rmSync(path, { force: true });
    }
  }
  throw new AlreadyRunningError("não foi possível obter a trava de instância");
}
