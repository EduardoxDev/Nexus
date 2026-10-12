import { describe, expect, it } from "vitest";

import { ROUTES } from "@/app/routes";

import { useUi } from "./ui";

describe("navegação", () => {
  it("todas as seções do roadmap da interface já existem e são navegáveis", () => {
    expect(ROUTES.every((r) => r.plannedPhase === null)).toBe(true);
    expect(useUi.getState().navigate("integrations")).toBe(true);
    expect(useUi.getState().route).toBe("integrations");
    useUi.getState().back();
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
