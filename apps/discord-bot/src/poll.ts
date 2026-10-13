/** Repete uma tarefa em intervalo fixo, sem sobrepor execuções; falhas vão para o log. */
export function every(
  intervalMs: number,
  task: () => Promise<void>,
  log: (line: string) => void,
  firstDelayMs = 10_000,
): () => void {
  let running = false;
  const run = async () => {
    if (running) return;
    running = true;
    try {
      await task();
    } catch (e) {
      const err = e as Error;
      log(`${err.name}: ${err.message}`);
    } finally {
      running = false;
    }
  };
  const first = setTimeout(() => void run(), firstDelayMs);
  const timer = setInterval(() => void run(), intervalMs);
  return () => {
    clearTimeout(first);
    clearInterval(timer);
  };
}
