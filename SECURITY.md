# Segurança

## Como relatar uma vulnerabilidade

**Não abra uma issue pública.** Use o relato privado do GitHub (aba **Security → Report a vulnerability** do repositório). Inclua versão, passos para reproduzir e o impacto esperado. Respondemos em até 7 dias e combinamos a divulgação depois da correção.

## Versões suportadas

Só a versão mais recente recebe correções de segurança (o projeto ainda não tem release publicada).

## O que o NEXUS protege

Resumo; o detalhamento, com ameaças e testes, está no [modelo de ameaças](docs/threat-model.md).

- **Nada sai da máquina**: sem telemetria, sem rede no core; a CSP do app não libera nenhuma origem remota.
- **Webview isolado**: sem acesso a arquivos, shell ou HTTP; comandos em lista explícita (testada); navegação para fora do app e novas janelas são recusadas; CSP com Trusted Types (nada de HTML por string).
- **Projetos autorizados**: só pastas escolhidas pelo usuário; links e junctions nunca são seguidos, nem na indexação nem nas releituras; segredos e arquivos sensíveis não têm conteúdo indexado nem exibido, inclusive quando aparecem depois da indexação.
- **Agentes (MCP)**: escopo restrito aos projetos autorizados, sem escrita em arquivos de projeto, conteúdo devolvido marcado como dado (não instrução), auditoria de cada chamada sem registrar argumentos.
- **Cadeia de suprimentos**: `Cargo.lock` e `pnpm-lock.yaml` versionados; CI com `cargo deny` (RustSec, licenças, origens), `pnpm audit`, CodeQL e Dependabot.
- **Código nativo**: `unsafe` proibido em todo o workspace, com uma única exceção documentada (cor da barra de título via DWM, `apps/desktop/src-tauri/src/chrome.rs`).

## Pendências conhecidas

- Instalador e executáveis **não assinados** (certificado de assinatura de código pendente).
- Sem atualização automática (chaves e endpoint pendentes).
