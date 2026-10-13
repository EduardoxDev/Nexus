/**
 * Versões novas da NEXUS: consulta a última release publicada no GitHub (API pública, sem
 * token; rascunhos não aparecem) e decide se precisa anunciar.
 */
import type { BotState } from "./state.ts";

export interface ReleaseAsset {
  name: string;
  url: string;
}

export interface Release {
  tag: string;
  name: string;
  notes: string;
  url: string;
  assets: ReleaseAsset[];
}

export type FetchFn = (url: string, init?: RequestInit) => Promise<Response>;

const ANNOUNCED = "release.announced";

/** Última release publicada, ou `null` se o repositório ainda não tem nenhuma. */
export async function latestRelease(fetchFn: FetchFn, repo: string): Promise<Release | null> {
  const res = await fetchFn(`https://api.github.com/repos/${repo}/releases/latest`, {
    headers: {
      Accept: "application/vnd.github+json",
      "User-Agent": "nexus-discord-bot",
      "X-GitHub-Api-Version": "2022-11-28",
    },
    signal: AbortSignal.timeout(15_000),
  });
  if (res.status === 404) return null;
  if (!res.ok) throw new Error(`o GitHub respondeu ${res.status}`);
  const r = (await res.json()) as {
    tag_name: string;
    name: string | null;
    body: string | null;
    html_url: string;
    assets?: { name: string; browser_download_url: string }[];
  };
  return {
    tag: r.tag_name,
    name: r.name || r.tag_name,
    notes: r.body ?? "",
    url: r.html_url,
    assets: (r.assets ?? []).map((a) => ({ name: a.name, url: a.browser_download_url })),
  };
}

/**
 * Se `r` ainda não foi anunciada. Na primeira execução só registra a versão atual (sem anunciar
 * uma release antiga ao instalar o bot); dali em diante, cada tag nova é anunciada uma vez.
 */
export function shouldAnnounce(state: BotState, r: Release): "first-run" | "new" | "known" {
  const last = state.get(ANNOUNCED);
  if (last === r.tag) return "known";
  state.set(ANNOUNCED, r.tag);
  return last === null ? "first-run" : "new";
}

/** Downloads por sistema, na ordem em que aparecem no anúncio. */
export function downloads(assets: ReleaseAsset[]): { label: string; url: string }[] {
  const pick = (label: string, test: (n: string) => boolean) => {
    const a = assets.find((x) => test(x.name));
    return a ? [{ label, url: a.url }] : [];
  };
  return [
    ...pick("Windows", (n) => n.endsWith("-setup.exe")),
    ...pick("macOS (Apple Silicon)", (n) => n.endsWith("_aarch64.dmg")),
    ...pick("macOS (Intel)", (n) => n.endsWith("_x64.dmg")),
    ...pick("Linux (.AppImage)", (n) => n.endsWith(".AppImage")),
    ...pick("Linux (.deb)", (n) => n.endsWith(".deb")),
  ];
}
