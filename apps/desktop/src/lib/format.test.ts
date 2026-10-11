import { describe, expect, it } from "vitest";

import { formatBytes, formatDuration, formatRelative, shortHash } from "./format";

describe("format", () => {
  it("formata bytes em unidades binárias com vírgula decimal", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1023)).toBe("1.023 B");
    expect(formatBytes(2048)).toBe("2,0 KiB");
    expect(formatBytes(5 * 1024 * 1024)).toBe("5,0 MiB");
  });

  it("formata durações curtas e longas", () => {
    expect(formatDuration(12)).toBe("12 ms");
    expect(formatDuration(1500)).toBe("1,5 s");
    expect(formatDuration(125_000)).toBe("2 min 5 s");
  });

  it("formata tempo relativo em pt-BR", () => {
    const now = 1_000_000_000;
    expect(formatRelative(now - 5 * 60_000, now)).toBe("há 5 minutos");
    expect(formatRelative(now - 24 * 3_600_000, now)).toBe("ontem");
  });

  it("encurta hashes", () => {
    expect(shortHash("abcdef0123456789")).toBe("abcdef012345");
  });
});
