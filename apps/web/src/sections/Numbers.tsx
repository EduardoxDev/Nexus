import { CountUp, Reveal } from "@/components/fx";

/** Medições reais registradas em docs/benchmarks.md e docs/evaluation.md (Windows, 2026-10). */
const STATS = [
  { value: 3, decimals: 0, prefix: "~", suffix: " s", label: "para indexar 10 mil arquivos" },
  { value: 0.4, decimals: 1, suffix: " s", label: "para atualizar depois de uma mudança" },
  { value: 37, decimals: 0, suffix: " ms", label: "por busca (mediana)" },
  {
    value: 0.958,
    decimals: 3,
    label: "de qualidade da busca (MRR, máximo 1), contra 0,668 da busca comum",
  },
];

export function Numbers() {
  return (
    <section className="relative py-20">
      <div className="mx-auto max-w-6xl px-5">
        <Reveal className="overflow-hidden rounded-3xl border border-border bg-card">
          <div className="grid divide-y divide-border sm:grid-cols-2 sm:divide-y-0 lg:grid-cols-4 lg:divide-x">
            {STATS.map((s) => (
              <div key={s.label} className="p-8 text-center sm:border-border lg:border-0">
                <div className="font-display text-5xl font-bold tracking-tight">
                  <CountUp
                    to={s.value}
                    decimals={s.decimals}
                    prefix={s.prefix ?? ""}
                    suffix={s.suffix ?? ""}
                    className="text-gradient"
                  />
                </div>
                <p className="mt-2 text-sm text-muted-foreground">{s.label}</p>
              </div>
            ))}
          </div>
          <p className="border-t border-border px-6 py-3 text-center text-xs text-muted-foreground">
            Medido num PC comum com Windows, em repositórios reais e sintéticos. Os scripts estão no
            repositório — dá para reproduzir.
          </p>
        </Reveal>
      </div>
    </section>
  );
}
