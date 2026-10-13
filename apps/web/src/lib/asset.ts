/** Caminho de um arquivo de public/ respeitando a base do site (/ local, /Nexus/ no GitHub Pages). */
export function asset(path: string): string {
  return import.meta.env.BASE_URL + path.replace(/^\//, "");
}
