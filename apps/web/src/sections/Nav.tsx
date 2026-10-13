import { Download, Moon, Sun } from "lucide-react";
import { motion, useScroll, useTransform } from "motion/react";
import { useEffect, useState } from "react";

import { GithubIcon } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { downloadHref, site } from "@/site";

const LINKS = [
  { href: "#recursos", label: "Recursos" },
  { href: "#como-funciona", label: "Como funciona" },
  { href: "#privacidade", label: "Privacidade" },
  { href: "#faq", label: "Perguntas" },
];

export function Logo({ className = "size-7" }: { className?: string }) {
  return (
    <img
      src="/logo.png"
      alt=""
      width={28}
      height={28}
      className={`${className} rounded-lg shadow-sm`}
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
      // armazenamento indisponível: o tema vale só nesta visita
    }
  };
  return (
    <Button variant="ghost" size="icon" onClick={toggle} aria-label="Alternar tema claro/escuro">
      {dark ? <Sun /> : <Moon />}
    </Button>
  );
}

export function Nav() {
  const { scrollY } = useScroll();
  const border = useTransform(scrollY, [0, 80], ["oklch(0 0 0 / 0)", "var(--border)"]);
  return (
    <motion.header
      style={{ borderColor: border }}
      className="fixed inset-x-0 top-0 z-50 border-b bg-background/70 backdrop-blur-xl supports-[backdrop-filter]:bg-background/55"
    >
      <nav
        aria-label="Principal"
        className="mx-auto flex h-16 max-w-6xl items-center justify-between gap-4 px-5"
      >
        <a href="#" className="flex items-center gap-2.5" aria-label="NEXUS, início">
          <Logo />
          <span className="font-display text-[15px] font-bold tracking-[0.14em]">{site.name}</span>
        </a>
        <ul className="hidden items-center gap-1 md:flex">
          {LINKS.map((l) => (
            <li key={l.href}>
              <a
                href={l.href}
                className="rounded-md px-3 py-2 text-sm text-muted-foreground transition-colors hover:text-foreground"
              >
                {l.label}
              </a>
            </li>
          ))}
        </ul>
        <div className="flex items-center gap-1.5">
          <ThemeToggle />
          {site.githubUrl && (
            <Button variant="ghost" size="icon" asChild>
              <a href={site.githubUrl} aria-label="Código no GitHub" rel="noopener">
                <GithubIcon />
              </a>
            </Button>
          )}
          <Button size="sm" asChild>
            <a href={downloadHref || "#download"}>
              <Download />
              Baixar
            </a>
          </Button>
        </div>
      </nav>
    </motion.header>
  );
}
