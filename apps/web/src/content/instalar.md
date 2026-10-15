# Instalar a NEXUS

A NEXUS roda no Windows, no Linux e no macOS. Os pacotes ficam na [página da última versão](https://github.com/EduardoxDev/Nexus/releases/latest) no GitHub, junto com o arquivo `SHA256SUMS.txt` para conferir o download. Todos já trazem a linha de comando `nexus`, que os agentes usam.

Os pacotes ainda **não são assinados** por uma empresa certificadora. Por isso o sistema pode avisar na primeira abertura; abaixo está como abrir em cada um.

## Windows

1. Baixe `NEXUS_<versão>_x64-setup.exe`.
2. Abra o instalador. Ele instala só para o seu usuário e **não pede administrador**.
3. Se aparecer "O Windows protegeu o computador", clique em **Mais informações** → **Executar assim mesmo**.

A CLI `nexus.exe` fica ao lado do app, onde a tela **Integrações** a encontra. Desinstalar pelo Windows remove o programa e mantém seus dados em `%LOCALAPPDATA%\Nexus`.

## macOS

Há pacotes para **Apple Silicon** (`aarch64`) e **Intel** (`x64`). Baixe o `.dmg` do seu Mac e arraste a NEXUS para **Aplicativos**.

Como o app não tem assinatura da Apple, o macOS pode dizer que ele "está danificado" ou "não pode ser aberto". Para liberar, clique com o botão direito no app → **Abrir**, ou rode no Terminal:

```bash
xattr -dr com.apple.quarantine /Applications/NEXUS.app
```

A CLI fica em `NEXUS.app/Contents/MacOS/nexus`.

## Linux

Os pacotes Linux são para **x86_64** (64 bits Intel/AMD); ainda não há versão para ARM.

- **`.deb`** (Debian, Ubuntu e derivados): `sudo apt install ./nexus_<versão>_amd64.deb`. A CLI fica em `/usr/bin/nexus`.
- **`.AppImage`** (qualquer distribuição em x86_64): dê permissão de execução com `chmod +x` e abra o arquivo. Em distribuições recentes pode ser preciso instalar o `libfuse2`.

## Atualizações

Na primeira abertura, a NEXUS pergunta se pode verificar versões novas no GitHub. Sem resposta, ela não se conecta a nada. Com a sua permissão, a verificação automática acontece no máximo uma vez por dia. Em **Configurações**, **Verificar agora** consulta na hora, mesmo com a verificação automática desligada. A atualização só é instalada quando você clica, depois de conferida por assinatura.

Vale para o instalador do Windows, o `.AppImage` e o macOS. Quem instalou pelo `.deb` atualiza baixando o `.deb` novo. A escolha pode ser mudada a qualquer hora em **Configurações**. Mais detalhes em [Privacidade](/docs/privacidade/).

## Depois de instalar

1. Abra a NEXUS e adicione a pasta de um projeto. Ela indexa sozinha e deixa de fora senhas, chaves e binários.
2. Conecte o seu agente de IA: veja [Conectar seu agente](/docs/agentes/).
