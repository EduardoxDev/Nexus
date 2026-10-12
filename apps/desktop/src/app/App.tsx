import { DatabaseZap } from "lucide-react";

import { AppShell } from "@/app/AppShell";
import logoUrl from "@/assets/nexus-icon-128.png";
import { ErrorState } from "@/components/feedback";
import { KeyValueList } from "@/components/layout";
import type { StartupStatus } from "@/lib/bindings";
import { t } from "@/lib/i18n/pt-BR";
import { useSettings, useStartupStatus } from "@/lib/queries";

/** Porta de entrada: só mostra o shell quando o core abriu; caso contrário, explica o motivo. */
export function App() {
  const startup = useStartupStatus();
  const ready = startup.data?.ready === true;
  const settings = useSettings();

  if (startup.isError) {
    return (
      <Centered>
        <ErrorState error={startup.error} onRetry={() => void startup.refetch()} />
      </Centered>
    );
  }
  if (startup.data && !startup.data.ready) {
    return <StartupFailure status={startup.data} />;
  }
  if (!ready || !settings.data) {
    if (settings.isError) {
      return (
        <Centered>
          <ErrorState error={settings.error} onRetry={() => void settings.refetch()} />
        </Centered>
      );
    }
    return <Splash />;
  }
  return <AppShell settings={settings.data} />;
}

/** Mesma tela de abertura do index.html (estilos `.nx-splash` definidos lá), sem salto visual. */
function Splash() {
  return (
    <div className="nx-splash" aria-busy="true" aria-label={t.common.loading}>
      <img src={logoUrl} alt="" draggable={false} />
      <p>{t.app.name}</p>
      <span />
    </div>
  );
}

function Centered({ children }: { children: React.ReactNode }) {
  return <div className="flex h-full items-center justify-center p-8">{children}</div>;
}

function StartupFailure({ status }: { status: StartupStatus }) {
  const incompatible = status.error?.code === "incompatible_schema";
  return (
    <Centered>
      <div className="w-full max-w-xl rounded-lg border border-border bg-surface-1 p-6">
        <DatabaseZap className="size-6 text-danger" aria-hidden="true" />
        <h1 className="mt-3 text-xl font-semibold">{t.startup.title}</h1>
        <p className="mt-1 text-sm text-fg-muted">{t.startup.description}</p>
        {status.error && <ErrorState className="mt-4" error={status.error} />}
        <p className="mt-4 text-sm text-fg">
          {incompatible ? t.startup.incompatibleHint : t.startup.genericHint}
        </p>
        <KeyValueList
          className="mt-4"
          items={[
            { label: t.startup.dataDir, value: status.dataDir ?? t.common.unknown, mono: true },
          ]}
        />
      </div>
    </Centered>
  );
}
