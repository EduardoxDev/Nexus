/**
 * Configuração do site. Os links vazios escondem os botões correspondentes até serem preenchidos
 * (nada de link quebrado ou "em breve" falso).
 */
export const site = {
  name: "NEXUS",
  version: "0.1.0",
  tagline: "Contexto local para engenharia com IA",
  slogan: "Conectando ideias, criando o amanhã.",
  description:
    "A NEXUS indexa seus projetos no seu computador e entrega aos seus agentes de IA busca com evidências, Context Packs e a memória do projeto — sem nada sair da sua máquina.",
  /** Repositório no GitHub, ex.: "https://github.com/usuario/nexus". */
  githubUrl: "",
  /** Download do instalador; padrão: a última release do GitHub. */
  downloadUrl: "",
  /** Quem fez: aparece na seção "Feito com o coração" e no rodapé. */
  maker: { name: "", url: "" },
} as const;

export const downloadHref =
  site.downloadUrl || (site.githubUrl ? `${site.githubUrl}/releases/latest` : "");
