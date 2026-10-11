import { useEffect, useState } from "react";

/** Hook React que consulta se há sessão ativa no navegador. */
export function useSession() {
  const [authenticated, setAuthenticated] = useState<boolean | null>(null);
  useEffect(() => {
    void fetch("/api/orders").then((r) => setAuthenticated(r.status !== 401));
  }, []);
  return { authenticated };
}
