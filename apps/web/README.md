# Site da NEXUS

Página de apresentação e download. Vite + React 19 + Tailwind v4, componentes no padrão
[shadcn/ui](https://ui.shadcn.com) (Radix) e efeitos no estilo [React Bits](https://reactbits.dev)
escritos em `src/components/fx` (Motion, sem dependências extras). O build pré-renderiza a página
(`scripts/prerender.mjs`): o HTML já traz todo o conteúdo e o React só assume a página.

```bash
corepack pnpm --filter @nexus/web dev      # http://127.0.0.1:5180
corepack pnpm --filter @nexus/web test
corepack pnpm --filter @nexus/web build    # gera dist/ (site estático)
```

## Antes de publicar

Preencha `src/site.ts`:

- `githubUrl` — mostra os botões do GitHub e aponta o download para a última release;
- `maker` — nome (e link) de quem fez, na seção "Feito com o coração" e no rodapé.

Enquanto estiverem vazios, os botões correspondentes não aparecem (nada de link quebrado).

`dist/` é um site estático: serve no GitHub Pages, Vercel, Netlify ou Cloudflare Pages.

Regras do conteúdo: só números medidos (fontes em `docs/benchmarks.md` e `docs/evaluation.md`) e
telas reais do app (`docs/assets`, regeneradas com `pnpm --filter @nexus/desktop screens`).
