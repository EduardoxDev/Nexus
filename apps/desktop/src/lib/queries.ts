/** Hooks TanStack Query sobre os comandos do core. Chaves centralizadas para invalidação. */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import type { OperationQuery, SettingKey } from "@/lib/bindings";
import { call } from "@/lib/ipc";

export const queryKeys = {
  startup: ["startup"] as const,
  appInfo: ["appInfo"] as const,
  settings: ["settings"] as const,
  diagnostics: ["diagnostics"] as const,
  operations: (query: OperationQuery) => ["operations", query] as const,
  operationsAll: ["operations"] as const,
};

export function useStartupStatus() {
  return useQuery({
    queryKey: queryKeys.startup,
    queryFn: () => call("get_startup_status"),
    staleTime: Infinity,
  });
}

export function useAppInfo() {
  return useQuery({
    queryKey: queryKeys.appInfo,
    queryFn: () => call("get_app_info"),
    staleTime: Infinity,
  });
}

export function useSettings() {
  return useQuery({ queryKey: queryKeys.settings, queryFn: () => call("get_settings") });
}

export function useSetSetting() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ key, value }: { key: SettingKey; value: unknown }) =>
      call("set_setting", { key, value }),
    onSuccess: (settings) => {
      client.setQueryData(queryKeys.settings, settings);
      void client.invalidateQueries({ queryKey: queryKeys.operationsAll });
    },
  });
}

export function useDiagnostics() {
  return useQuery({ queryKey: queryKeys.diagnostics, queryFn: () => call("get_diagnostics") });
}

export function useOperations(query: OperationQuery, refetchIntervalMs?: number) {
  return useQuery({
    queryKey: queryKeys.operations(query),
    queryFn: () => call("list_operations", { query }),
    ...(refetchIntervalMs === undefined ? {} : { refetchInterval: refetchIntervalMs }),
  });
}

export function useIntegrityCheck() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: () => call("run_integrity_check"),
    onSettled: () => {
      void client.invalidateQueries({ queryKey: queryKeys.diagnostics });
      void client.invalidateQueries({ queryKey: queryKeys.operationsAll });
    },
  });
}

export function useRevealDataDir() {
  return useMutation({ mutationFn: () => call("reveal_data_dir") });
}

/** Consulta padrão de operações (todos os campos explícitos, como exige o contrato). */
export function operationQuery(partial: Partial<OperationQuery> = {}): OperationQuery {
  return { limit: null, kindPrefix: null, status: null, ...partial };
}
