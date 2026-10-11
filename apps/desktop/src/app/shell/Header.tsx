import { ArrowLeft, ArrowRight, ChevronRight, Moon, Search, Sun } from "lucide-react";

import { routeDef } from "@/app/routes";
import { Button } from "@/components/ui/button";
import { Kbd } from "@/components/ui/kbd";
import { Tooltip } from "@/components/ui/tooltip";
import { t } from "@/lib/i18n/pt-BR";
import type { ResolvedTheme } from "@/lib/theme";
import { useUi } from "@/stores/ui";

interface HeaderProps {
  resolvedTheme: ResolvedTheme;
  onToggleTheme: () => void;
}

export function Header({ resolvedTheme, onToggleTheme }: HeaderProps) {
  const route = useUi((s) => s.route);
  const canBack = useUi((s) => s.past.length > 0);
  const canForward = useUi((s) => s.future.length > 0);
  const { back, forward, navigate, setPaletteOpen } = useUi.getState();
  const current = routeDef(route);

  return (
    <header className="flex h-12 shrink-0 items-center gap-3 border-b border-border px-3">
      <div className="flex items-center gap-0.5">
        <Tooltip content={`${t.nav.back} (Alt+←)`}>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={back}
            disabled={!canBack}
            aria-label={t.nav.back}
          >
            <ArrowLeft />
          </Button>
        </Tooltip>
        <Tooltip content={`${t.nav.forward} (Alt+→)`}>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={forward}
            disabled={!canForward}
            aria-label={t.nav.forward}
          >
            <ArrowRight />
          </Button>
        </Tooltip>
      </div>

      <nav aria-label="Trilha" className="min-w-0 flex-1">
        <ol className="flex items-center gap-1 text-sm">
          <li>
            {route === "overview" ? (
              <span className="text-fg-muted">{t.nav.home}</span>
            ) : (
              <button
                type="button"
                onClick={() => navigate("overview")}
                className="rounded-sm text-fg-muted hover:text-fg"
              >
                {t.nav.home}
              </button>
            )}
          </li>
          {route !== "overview" && (
            <>
              <li aria-hidden="true">
                <ChevronRight className="size-3.5 text-fg-subtle" />
              </li>
              <li aria-current="page" className="truncate font-medium text-fg">
                {current.label}
              </li>
            </>
          )}
        </ol>
      </nav>

      <button
        type="button"
        onClick={() => setPaletteOpen(true)}
        aria-label={t.header.openPalette}
        className="flex h-8 w-64 items-center gap-2 rounded-md border border-border bg-surface-2 px-2.5 text-sm text-fg-subtle transition-colors hover:border-border-strong hover:text-fg-muted"
      >
        <Search className="size-4" aria-hidden="true" />
        <span className="flex-1 text-left">{t.header.paletteHint}</span>
        <span className="flex gap-0.5">
          <Kbd>Ctrl</Kbd>
          <Kbd>K</Kbd>
        </span>
      </button>

      <Tooltip content={t.header.toggleTheme}>
        <Button
          variant="ghost"
          size="icon-sm"
          onClick={onToggleTheme}
          aria-label={t.header.toggleTheme}
        >
          {resolvedTheme === "dark" ? <Sun /> : <Moon />}
        </Button>
      </Tooltip>
    </header>
  );
}
