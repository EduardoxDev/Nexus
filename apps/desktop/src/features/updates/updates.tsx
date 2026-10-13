/**
 * Atualizações: a única conexão de rede do app, sempre com permissão da pessoa.
 * A consulta e a instalação são comandos do Rust (o webview não acessa a rede).
 */
import { useMutation } from "@tanstack/react-query";
import { Download, RefreshCw } from "lucide-react";
import { useEffect, useRef } from "react";
import { create } from "zustand";

import { ErrorState } from "@/components/feedback";
import { Panel } from "@/components/layout";
import { Button } from "@/components/ui/button";
import { Segmented, type SegmentedOption } from "@/components/ui/segmented";
import type { Settings, UpdateInfo, UpdateStatus } from "@/lib/bindings";
import { t } from "@/lib/i18n/pt-BR";
import { call } from "@/lib/ipc";
import { useSetSetting } from "@/lib/queries";
import { useToasts } from "@/stores/toast";

const s = t.updates;

/** Versão nova encontrada nesta sessão (pela verificação ao abrir ou pelo botão). */
export const useFoundUpdate = create<{
  found: UpdateInfo | null;
  setFound: (u: UpdateInfo | null) => void;
}>()((set) => ({ found: null, setFound: (found) => set({ found }) }));

function remember(status: UpdateStatus): void {
  useFoundUpdate.getState().setFound(status.status === "available" ? status : null);
}

/** Ao abrir o app: verifica só se a pessoa permitiu (o Rust ainda limita a 1×/dia). */
export function useStartupUpdateCheck(settings: Settings): void {
  const started = useRef(false);
  useEffect(() => {
    if (started.current || settings.updateCheck !== "on") return;
    started.current = true;
    call("check_for_update", { manual: false })
      .then((status) => {
        remember(status);
        if (status.status === "available") {
          useToasts.getState().push({
            tone: "info",
            title: s.toastTitle(status.version),
            action: { label: s.toastAction, route: "settings" },
          });
        }
      })
      // Sem rede ou GitHub fora do ar: a verificação automática falha em silêncio (fica no log).
      .catch(() => undefined);
  }, [settings.updateCheck]);
}

/** Pergunta na Visão geral enquanto a preferência estiver sem resposta. */
export function UpdateConsent({ settings }: { settings: Settings }) {
  const setSetting = useSetSetting();
  if (settings.updateCheck !== "ask") return null;
  const answer = (value: "on" | "off") => setSetting.mutate({ key: "updates.check", value });
  return (
    <Panel title={s.consentTitle}>
      <div className="flex flex-col gap-3">
        <p className="text-sm text-fg-muted">{s.consentBody}</p>
        <div className="flex gap-2">
          <Button variant="primary" onClick={() => answer("on")} disabled={setSetting.isPending}>
            {s.consentAllow}
          </Button>
          <Button variant="ghost" onClick={() => answer("off")} disabled={setSetting.isPending}>
            {s.consentDeny}
          </Button>
        </div>
        {setSetting.isError && <ErrorState error={setSetting.error} />}
      </div>
    </Panel>
  );
}

const AUTO_OPTIONS: readonly SegmentedOption<"on" | "off">[] = [
  { value: "on", label: s.on },
  { value: "off", label: s.off },
];

/** Painel em Configurações: preferência, "Verificar agora" e instalação. */
export function UpdatesPanel({ settings }: { settings: Settings }) {
  const setSetting = useSetSetting();
  const found = useFoundUpdate((st) => st.found);
  const check = useMutation({
    mutationFn: () => call("check_for_update", { manual: true }),
    onSuccess: remember,
  });
  const install = useMutation({ mutationFn: () => call("install_update") });
  const upToDate = check.data?.status === "upToDate" ? check.data.currentVersion : null;

  return (
    <Panel title={s.title} description={s.description}>
      <div className="flex flex-col gap-4">
        <div className="flex flex-wrap items-center justify-between gap-x-6 gap-y-2">
          <div className="min-w-0 max-w-md">
            <p className="text-sm font-medium text-fg">{s.auto}</p>
            <p className="text-xs text-fg-muted">{s.autoDescription}</p>
          </div>
          <Segmented
            aria-label={s.auto}
            value={settings.updateCheck === "on" ? "on" : "off"}
            options={AUTO_OPTIONS}
            disabled={setSetting.isPending}
            onValueChange={(value) => setSetting.mutate({ key: "updates.check", value })}
          />
        </div>

        <div className="flex flex-wrap items-center gap-3">
          <Button onClick={() => check.mutate()} disabled={check.isPending || install.isPending}>
            <RefreshCw aria-hidden="true" />
            {check.isPending ? s.checking : s.checkNow}
          </Button>
          {upToDate && !found && (
            <p role="status" className="text-sm text-fg-muted">
              {s.upToDate(upToDate)}
            </p>
          )}
        </div>

        {found && (
          <div
            role="region"
            aria-label={s.available(found.version)}
            className="flex flex-col gap-2 rounded-md border border-border bg-surface-2 p-3"
          >
            <p className="text-sm font-medium text-fg">{s.available(found.version)}</p>
            <p className="text-xs text-fg-muted">{s.current(found.currentVersion)}</p>
            {found.notes && (
              <pre className="selectable max-h-48 overflow-auto font-sans text-xs whitespace-pre-wrap text-fg">
                {found.notes}
              </pre>
            )}
            <div className="flex flex-wrap items-center gap-3">
              <Button
                variant="primary"
                onClick={() => install.mutate()}
                disabled={install.isPending}
              >
                <Download aria-hidden="true" />
                {install.isPending ? s.installing : s.install}
              </Button>
              <p className="text-xs text-fg-muted">{s.installHint}</p>
            </div>
          </div>
        )}

        {check.isError && <ErrorState error={check.error} />}
        {install.isError && <ErrorState error={install.error} />}
        {setSetting.isError && <ErrorState error={setSetting.error} />}
      </div>
    </Panel>
  );
}
