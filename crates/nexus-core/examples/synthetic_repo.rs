//! Gera um repositório TypeScript sintético e determinístico para benchmarks.
//!
//! ```text
//! cargo run --release -p nexus-core --example synthetic_repo -- <pasta> [arquivos=10000]
//! ```
//!
//! Cada pacote `pkgNNN/` tem 100 módulos; cada módulo importa o anterior (mesmo pacote ou o
//! último do pacote anterior) e `../shared/util.ts`, então o grafo tem arestas que resolvem.

use std::path::PathBuf;

fn module(i: usize) -> String {
    let import_prev = if i == 0 {
        String::new()
    } else if (i - 1) / 100 == i / 100 {
        format!("import {{ helper{p} }} from \"./m{p}\";\n", p = i - 1)
    } else {
        format!(
            "import {{ helper{p} }} from \"../pkg{:03}/m{p}\";\n",
            (i - 1) / 100,
            p = i - 1
        )
    };
    format!(
        "// módulo {i}\n{import_prev}import {{ round }} from \"../shared/util\";\n\n\
         export interface Record{i} {{\n  id: string;\n  total{i}: number;\n}}\n\n\
         /** Soma os totais dos registros do módulo {i}. */\n\
         export function computeTotal{i}(items: Record{i}[]): number {{\n\
         \x20 return round(items.reduce((acc, item) => acc + item.total{i}, 0));\n}}\n\n\
         export class Store{i} {{\n  private items: Record{i}[] = [];\n\n\
         \x20 add(item: Record{i}): void {{\n    this.items.push(item);\n  }}\n}}\n\n\
         export const helper{i} = (value: number) => value * {i};\n"
    )
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(dir) = args.next().map(PathBuf::from) else {
        eprintln!("uso: synthetic_repo <pasta> [arquivos]");
        std::process::exit(2);
    };
    let files: usize = args.next().and_then(|n| n.parse().ok()).unwrap_or(10_000);
    let write = |path: PathBuf, text: String| {
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            eprintln!("erro ao criar {}: {e}", parent.display());
            std::process::exit(1);
        }
        if let Err(e) = std::fs::write(&path, text) {
            eprintln!("erro ao gravar {}: {e}", path.display());
            std::process::exit(1);
        }
    };
    write(
        dir.join("shared/util.ts"),
        "export function round(x: number): number {\n  return Math.round(x * 100) / 100;\n}\n"
            .into(),
    );
    write(
        dir.join("README.md"),
        "# Repositório sintético\n\nVeja `shared/util.ts`.\n".into(),
    );
    for i in 0..files {
        write(dir.join(format!("pkg{:03}/m{i}.ts", i / 100)), module(i));
    }
    println!("{} arquivos em {}", files + 2, dir.display());
}
