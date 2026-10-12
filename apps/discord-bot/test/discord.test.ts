import { Events, Partials } from "discord.js";
import { describe, expect, it } from "vitest";

import { channelRef, createClient } from "../src/discord.ts";

type Arg = Parameters<typeof channelRef>[0];
const interaction = (channel: unknown, channelId = "555") =>
  ({ channelId, channel }) as unknown as Arg;

describe("ponte com o discord.js", () => {
  it("a thread inherits its parent channel", () => {
    const thread = { isThread: () => true, parentId: "100" };
    expect(channelRef(interaction(thread))).toEqual({ id: "555", parentId: "100" });
  });

  it("a channel's category never counts as parent", () => {
    const text = { isThread: () => false, parentId: "categoria-1" };
    expect(channelRef(interaction(text))).toEqual({ id: "555", parentId: null });
  });

  it("uncached channels are resolved as partials (private threads keep their parent)", () => {
    const client = createClient();
    expect(client.options.partials).toContain(Partials.Channel);
    void client.destroy();
  });

  it("gateway errors are handled instead of crashing the bot", () => {
    const client = createClient();
    expect(client.listenerCount(Events.Error)).toBe(1);
    void client.destroy();
  });
});
