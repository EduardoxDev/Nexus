import { describe, expect, it } from "vitest";

import { useUi } from "./ui";

describe("navegação", () => {
  it("recusa seções que ainda não existem", () => {
    expect(useUi.getState().navigate("projects")).toBe(false);
    expect(useUi.getState().route).toBe("overview");
  });

  it("mantém histórico de voltar/avançar", () => {
    const ui = useUi.getState();
    ui.navigate("settings");
    ui.navigate("diagnostics");
    useUi.getState().back();
    expect(useUi.getState().route).toBe("settings");
    useUi.getState().back();
    expect(useUi.getState().route).toBe("overview");
    useUi.getState().forward();
    expect(useUi.getState().route).toBe("settings");
    // Navegar após voltar descarta o "avançar".
    useUi.getState().navigate("overview");
    expect(useUi.getState().future).toEqual([]);
  });

  it("navegar para a rota atual não duplica o histórico", () => {
    useUi.getState().navigate("overview");
    expect(useUi.getState().past).toEqual([]);
  });
});
