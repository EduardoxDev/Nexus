import { describe, expect, it } from "vitest";

import { downloads, latestRelease, shouldAnnounce, type FetchFn } from "../src/releases.ts";
import { BotState } from "../src/state.ts";

const API = {
  tag_name: "v0.2.0",
  name: "NEXUS v0.2.0",
  body: "Novidades: Go, Java e C#.",
  html_url: "https://github.com/EduardoxDev/Nexus/releases/tag/v0.2.0",
  assets: [
    "NEXUS_0.2.0_x64-setup.exe",
    "NEXUS_0.2.0_x64-setup.exe.sig",
    "NEXUS_0.2.0_aarch64.dmg",
    "NEXUS_0.2.0_x64.dmg",
    "NEXUS_0.2.0_amd64.AppImage",
    "nexus_0.2.0_amd64.deb",
    "latest.json",
    "SHA256SUMS.txt",
  ].map((name) => ({ name, browser_download_url: `https://dl/${name}` })),
};

function fakeFetch(status: number, body: unknown = API) {
  const urls: string[] = [];
  const fetchFn: FetchFn = async (url) => {
    urls.push(url);
    return new Response(JSON.stringify(body), { status });
  };
  return { fetchFn, urls };
}

describe("releases do GitHub", () => {
  it("reads the latest published release", async () => {
    const { fetchFn, urls } = fakeFetch(200);
    const r = await latestRelease(fetchFn, "EduardoxDev/Nexus");
    expect(urls).toEqual(["https://api.github.com/repos/EduardoxDev/Nexus/releases/latest"]);
    expect(r?.tag).toBe("v0.2.0");
    expect(r?.assets).toHaveLength(8);
  });

  it("no release yet is not an error", async () => {
    expect(await latestRelease(fakeFetch(404).fetchFn, "a/b")).toBeNull();
  });

  it("other failures are errors", async () => {
    await expect(latestRelease(fakeFetch(500).fetchFn, "a/b")).rejects.toThrow(/500/);
  });

  it("announces each new tag once; the first run only records", async () => {
    const s = new BotState(":memory:");
    const r = (await latestRelease(fakeFetch(200).fetchFn, "a/b"))!;
    expect(shouldAnnounce(s, r)).toBe("first-run");
    expect(shouldAnnounce(s, r)).toBe("known");
    expect(shouldAnnounce(s, { ...r, tag: "v0.2.1" })).toBe("new");
    expect(shouldAnnounce(s, { ...r, tag: "v0.2.1" })).toBe("known");
  });

  it("downloads per system, ignoring signatures and checksums", () => {
    const assets = API.assets.map((a) => ({ name: a.name, url: a.browser_download_url }));
    expect(downloads(assets).map((d) => d.label)).toEqual([
      "Windows",
      "macOS (Apple Silicon)",
      "macOS (Intel)",
      "Linux (.AppImage)",
      "Linux (.deb)",
    ]);
  });
});
