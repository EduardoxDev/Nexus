import { useEffect, useState } from "react";

/** Tema atual do site ("dark" | "light"), acompanhando a classe do <html> (o botão de tema muda). */
export function useTheme(): "dark" | "light" {
  const [theme, setTheme] = useState<"dark" | "light">("dark");
  useEffect(() => {
    const el = document.documentElement;
    const read = () => setTheme(el.classList.contains("dark") ? "dark" : "light");
    read();
    const obs = new MutationObserver(read);
    obs.observe(el, { attributes: true, attributeFilter: ["class"] });
    return () => obs.disconnect();
  }, []);
  return theme;
}
