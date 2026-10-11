# Solução de problemas

Comece pelo **Diagnóstico** (no app ou `nexus diagnostics`). Toda falha mostra um código e, quando aplicável, um `opId` que identifica a operação na lista de operações e nos logs (`%LOCALAPPDATA%\Nexus\logs\nexus.*.jsonl`).

| Sintoma                                                                    | Causa provável                                                        | O que fazer                                                                                                                    |
| -------------------------------------------------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Tela "O NEXUS não conseguiu abrir o banco local" com `incompatible_schema` | O banco foi criado por uma versão mais nova do NEXUS                  | Atualize o app, ou restaure um backup de `backups\` (ver [database-schema.md](database-schema.md#backup-e-restauração))        |
| Mesma tela com `storage`                                                   | Arquivo bloqueado por outro programa, disco cheio ou banco corrompido | Feche outros programas que usem o arquivo (antivírus, sincronizadores), libere espaço, rode `nexus diagnostics --check`        |
| "a migration N foi alterada depois de aplicada"                            | Binário modificado ou build inconsistente                             | Reinstale a versão oficial; não edite migrations já publicadas                                                                 |
| Verificação de integridade com problemas                                   | Corrupção (queda de energia, disco, sincronização em nuvem da pasta)  | Restaure o backup mais recente; não mantenha `%LOCALAPPDATA%\Nexus` em pastas sincronizadas (OneDrive, Dropbox)                |
| `SQLITE_BUSY` / operações lentas com app e CLI abertos                     | Escritas concorrentes                                                 | Normal por alguns segundos (espera de 5 s); se persistir, feche um dos processos e reporte com o `opId`                        |
| App não abre / tela branca                                                 | WebView2 ausente ou corrompido                                        | Reinstale o [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/); o instalador do NEXUS tenta baixá-lo |
| SmartScreen alerta ao instalar                                             | Instalador não assinado (ainda)                                       | Esperado nesta fase; ver [release.md](release.md)                                                                              |
| Segunda janela não abre                                                    | Instância única                                                       | Comportamento esperado: a janela existente é focada                                                                            |
| `pnpm` não encontrado                                                      | pnpm não instalado globalmente                                        | Use `corepack pnpm …` ou `corepack enable`                                                                                     |
| Banco em pasta de rede                                                     | SQLite em rede não é suportado (locks não confiáveis)                 | Use um disco local; `NEXUS_DATA_DIR` precisa ser local e absoluto                                                              |

## Indexação

| Sintoma                                                                     | Causa provável                                                                               | O que fazer                                                                                                    |
| --------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| "a pasta pessoal (ou uma pasta acima dela) não pode ser um projeto inteiro" | Proteção contra indexar o computador todo                                                    | Escolha a pasta do repositório                                                                                 |
| "a pasta se sobrepõe ao projeto…"                                           | Projetos aninhados duplicariam o índice                                                      | Use o projeto existente ou remova-o antes                                                                      |
| Arquivo esperado não aparece                                                | `.gitignore`, `.nexusignore` ou exclusão padrão (`node_modules`, `target`, `dist`, `build`…) | Confira as regras; links e junctions nunca são seguidos                                                        |
| Arquivo legítimo marcado como "Sensível"                                    | Nome ou conteúdo parecido com credencial (ex.: token no formato de chave AWS em testes)      | Mova o valor para fora do código ou monte-o em tempo de execução; a detecção é preventiva por design           |
| Erro `unstable` no histórico                                                | O arquivo mudou durante a leitura (ex.: sendo salvo)                                         | Nada: ele é tentado de novo na próxima indexação                                                               |
| "já existe uma indexação em andamento para este projeto"                    | O app e a CLI tentaram indexar o mesmo projeto                                               | Aguarde; jobs de processos encerrados são liberados após 2 minutos sem heartbeat                               |
| Alterações não aparecem sozinhas                                            | "Acompanhar alterações" desligado, ou pasta em rede/sincronizada que não emite eventos       | Ligue em Configurações; a verificação automática a cada 10 minutos e o botão "Atualizar índice" cobrem o resto |
| Indexação deixa a máquina lenta                                             | Muitas threads                                                                               | Reduza "Threads de indexação" em Configurações ou pause o job                                                  |

## Isolar um problema

```bash
set NEXUS_DATA_DIR=C:\temp\nexus-limpo      # PowerShell: $env:NEXUS_DATA_DIR="C:\temp\nexus-limpo"
nexus init
nexus diagnostics --check
```

Para logs mais detalhados: `NEXUS_LOG=debug` (sintaxe do `tracing` `EnvFilter`).
