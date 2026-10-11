import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState, type ReactNode } from "react";

import { TooltipProvider } from "@/components/ui/tooltip";

export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      // Dados locais: sem retries agressivos; erros aparecem rápido com o opId.
      queries: { retry: 1, staleTime: 5_000, refetchOnWindowFocus: true },
      mutations: { retry: false },
    },
  });
}

export function Providers({ children, client }: { children: ReactNode; client?: QueryClient }) {
  const [queryClient] = useState(() => client ?? createQueryClient());
  return (
    <QueryClientProvider client={queryClient}>
      <TooltipProvider delayDuration={400} skipDelayDuration={150}>
        {children}
      </TooltipProvider>
    </QueryClientProvider>
  );
}
