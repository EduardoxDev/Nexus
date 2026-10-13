import { Moon, Sun } from "lucide-react";
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

export function Nav() {
  return (
    <header className="sticky top-0 z-50 border-b border-border/70 bg-background/70 backdrop-blur-md">
      <nav
        aria-label="Principal"
        className="mx-auto flex h-16 max-w-6xl items-center justify-between gap-4 px-5"
      >
        <a href="#" className="flex items-center gap-2.5" aria-label="NEXUS, início">
          <Logo />
          <span className="font-titulo text-lg">Nexus</span>
        </a>
        <ul className="hidden items-center gap-7 text-[15px] md:flex">
          {LINKS.map((l) => (
            <li key={l.href}>
              <a
                href={l.href}
                className="text-muted-foreground transition-colors hover:text-foreground"
              >
                {l.label}
              </a>
            </li>
          ))}
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
          <Button size="sm" asChild className="ml-1">
            <a href={downloadHref || "#comecar"}>Baixar</a>
          </Button>
        </div>
      </nav>
    </header>
  );
}
