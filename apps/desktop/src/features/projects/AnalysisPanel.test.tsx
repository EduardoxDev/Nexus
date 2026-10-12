import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { createQueryClient, Providers } from "@/app/providers";
import { AnalysisPanel } from "@/features/projects/AnalysisPanel";
import { setupIpc } from "@/test/harness";

describe("Análise de código", () => {
  it("mostra o grafo, a cobertura por linguagem e o que não é analisado", async () => {
    setupIpc({
      get_analysis_overview: () => ({
        graph: {
          indexGeneration: 3,
          builtAt: Date.now(),
          durationMs: 12,
          files: 40,
          edges: 90,
          resolved: 70,
          external: 19,
          unresolved: 1,
        },
        languages: [
          {
            language: "typescript",
            files: 35,
            symbols: 120,
            imports: 85,
            resolved: 66,
            external: 18,
            unresolved: 1,
          },
        ],
        notAnalyzed: [{ name: "go", count: 4 }],
        mostImported: [{ name: "src/utils/money.ts", count: 6 }],
        externalPackages: [{ name: "react", count: 3 }],
        coverage: [
          {
            language: "typescript / tsx / javascript / jsx",
            symbols: ["function"],
            imports: ["import … from"],
            notCovered: ["funções aninhadas"],
          },
        ],
      }),
    });
    const client = createQueryClient();
    client.setDefaultOptions({ queries: { retry: false } });
    render(
      <Providers client={client}>
        <AnalysisPanel projectId={1} />
      </Providers>,
    );
    const table = await screen.findByRole("table", { name: "Análise de código" });
    const row = within(table).getByRole("row", { name: /typescript/ });
    expect(row).toHaveTextContent("35");
    expect(row).toHaveTextContent("120");
    expect(row).toHaveTextContent("66");
    expect(screen.getByText(/90 imports — 70 entre arquivos do projeto/)).toBeInTheDocument();
    expect(screen.getByText(/Sem analisador \(só busca textual\): go \(4\)/)).toBeInTheDocument();
    expect(screen.getByText("src/utils/money.ts")).toBeInTheDocument();
    expect(screen.getByText(/funções aninhadas/)).toBeInTheDocument();
  });
});
