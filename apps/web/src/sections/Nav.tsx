import { Download, Moon, Sun } from "lucide-react";
import { motion } from "motion/react";
import { useEffect, useState } from "react";

import { GithubIcon } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { downloadHref, site } from "@/site";

const LINKS = [
  { href: "#o-que-faz", label: "O que faz" },
  { href: "#comecar", label: "Como começar" },
  { href: "#privacidade", label: "Privacidade" },
  { href: "#perguntas", label: "Perguntas" },
];

export function Logo({ className = "size-8" }: { className?: string }) {
  return (
    <img
      src="/logo.png"
      alt=""
      width={32}
      height={32}
      className={`${className} rounded-[10px]`}
      draggable={false}
    />
  );
}

function ThemeToggle() {
  const [dark, setDark] = useState(true);
  useEffect(() => setDark(document.documentElement.classList.contains("dark")), []);
  const toggle = () => {
    const next = !dark;
    setDark(next);
    document.documentElement.classList.toggle("dark", next);
    try {
      localStorage.setItem("theme", next ? "dark" : "light");
    } catch {
      // sem armazenamento: o tema vale só nesta visita
    }
  };
  return (
    <Button
      variant="ghost"
      size="icon"
      onClick={toggle}
      aria-label={dark ? "Usar tema claro" : "Usar tema escuro"}
    >
      {dark ? <Sun /> : <Moon />}
    </Button>
  );
}

/** Seção visível agora (para marcar o link da navbar). */
function useActiveSection(ids: string[]) {
  const [active, setActive] = useState<string | null>(null);
  useEffect(() => {
    const seen = new Map<string, boolean>();
    const io = new IntersectionObserver(
      (entries) => {
        for (const e of entries) seen.set(e.target.id, e.isIntersecting);
        setActive(ids.find((id) => seen.get(id)) ?? null);
      },
      { rootMargin: "-45% 0px -50% 0px" },
    );
    for (const id of ids) {
      const el = document.getElementById(id);
      if (el) io.observe(el);
    }
    return () => io.disconnect();
  }, [ids]);
  return active;
}

const IDS = LINKS.map((l) => l.href.slice(1));

export function Nav() {
  const active = useActiveSection(IDS);
  const [hover, setHover] = useState<string | null>(null);
  const [scrolled, setScrolled] = useState(false);
  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 8);
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);
  const lit = hover ?? active;

  return (
    <header className="sticky top-0 z-50 px-4 pt-3">
      <nav
        aria-label="Principal"
        className={`mx-auto flex h-14 max-w-5xl items-center justify-between gap-4 rounded-2xl border pr-2 pl-3 transition-[background-color,border-color,box-shadow] duration-300 ${
          scrolled
            ? "border-border bg-background/75 shadow-[0_12px_32px_-16px_rgb(0_0_0/0.6)] backdrop-blur-xl"
            : "border-transparent bg-transparent"
        }`}
      >
        <a href="#" className="flex items-center gap-2.5" aria-label="NEXUS, início">
          <Logo className="size-7" />
          <span className="text-[15px] font-semibold tracking-tight">Nexus</span>
        </a>
        <ul className="hidden items-center text-sm md:flex" onMouseLeave={() => setHover(null)}>
          {LINKS.map((l) => {
            const id = l.href.slice(1);
            return (
              <li key={l.href} className="relative">
                {lit === id && (
                  <motion.span
                    layoutId="nav-pill"
                    aria-hidden="true"
                    className="absolute inset-0 rounded-lg bg-muted"
                    transition={{ type: "spring", stiffness: 420, damping: 34 }}
                  />
                )}
                <a
                  href={l.href}
                  onMouseEnter={() => setHover(id)}
                  onFocus={() => setHover(id)}
                  onBlur={() => setHover(null)}
                  aria-current={active === id ? "true" : undefined}
                  className={`relative block px-3.5 py-1.5 transition-colors ${
                    lit === id ? "text-foreground" : "text-muted-foreground hover:text-foreground"
                  }`}
                >
                  {l.label}
                </a>
              </li>
            );
          })}
        </ul>
        <div className="flex items-center gap-1">
          <ThemeToggle />
          {site.githubUrl && (
            <Button variant="ghost" size="icon" asChild>
              <a href={site.githubUrl} aria-label="Código no GitHub" rel="noopener">
                <GithubIcon />
              </a>
            </Button>
          )}
          <Button size="sm" asChild className="ml-1 h-9 rounded-lg px-4">
            <a href={downloadHref || "#comecar"}>
              <Download />
              Baixar
            </a>
          </Button>
        </div>
      </nav>
    </header>
  );
}
