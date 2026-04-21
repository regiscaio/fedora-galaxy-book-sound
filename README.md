<p align="center">
  <img src="assets/galaxybook-sound.svg" alt="Ícone do Galaxy Book Sound" width="112">
</p>

<h1 align="center">Galaxy Book Sound</h1>

<p align="center">
  <a href="README.md">🇧🇷 Português</a>
  <a href="README.en.md">🇺🇸 English</a>
  <a href="README.es.md">🇪🇸 Español</a>
  <a href="README.it.md">🇮🇹 Italiano</a>
</p>

`Galaxy Book Sound` é um app nativo de GTK4 e libadwaita para ajuste de som no
Galaxy Book4 Ultra com Fedora. O foco dele é enxuto: equalizador de 10 bandas,
perfis prontos e um toggle de `Atmos compatível` aplicado nativamente via
`PipeWire` e `WirePlumber`.

A interface segue o padrão de preferências nativo do GNOME em uma página única,
com seleção de perfil base, toggle de `Atmos compatível`, equalizador e ações
de aplicação agrupadas.

## Escopo

Este app faz:

- carregar perfis `Neutro`, `Música` e `Cinema`;
- ajustar 10 bandas manualmente;
- ativar e desativar o modo `Atmos compatível`;
- gravar e aplicar o pipeline do app na sessão de áudio.

Este app não faz:

- instalar driver, módulos ou serviços do stack de áudio;
- diagnosticar hardware ou PipeWire;
- ativar Dolby Atmos proprietário.

Diagnóstico e instalação ficam no
[`fedora-galaxy-book-setup`](https://github.com/regiscaio/fedora-galaxy-book-setup).
O suporte MAX98390 continua no
[`fedora-galaxy-book-max98390`](https://github.com/regiscaio/fedora-galaxy-book-max98390).

## PipeWire

O app agora grava um `filter-chain` próprio em `~/.config/pipewire/pipewire.conf.d/`
e o aplica pela política de `smart filters` do `WirePlumber`, direcionado ao
sink interno do notebook.

Na prática isso significa:

- backend mantido pelo próprio app;
- configuração persistida pelo próprio app;
- aplicação transparente na saída interna após reiniciar a sessão de áudio.

## Instalação

Para instalar pelo repositório DNF público:

```bash
sudo dnf config-manager addrepo --from-repofile=https://packages.caioregis.com/fedora/caioregis.repo
sudo dnf install galaxybook-sound
```

Se quiser o fluxo completo do notebook, incluindo stack de áudio e diagnósticos,
use o `fedora-galaxy-book-setup`.

## Build

Dependências de build no Fedora:

```bash
sudo dnf install cargo rust pkgconf-pkg-config gtk4-devel libadwaita-devel
```

Comandos principais:

```bash
make build
make test
make smoke-test
make rpm
```

## Empacotamento

Arquivos relevantes:

- spec RPM: [`packaging/fedora/galaxybook-sound.spec`](packaging/fedora/galaxybook-sound.spec)
- launcher: [`data/com.caioregis.GalaxyBookSound.desktop`](data/com.caioregis.GalaxyBookSound.desktop)
- metadados AppStream: [`data/com.caioregis.GalaxyBookSound.metainfo.xml`](data/com.caioregis.GalaxyBookSound.metainfo.xml)

O pacote RPM acompanha só o que o app realmente usa, porque o backend de áudio
agora é um `filter-chain` próprio em `PipeWire`.

## Licença

Este projeto é distribuído sob a licença **GPL-3.0-only**. Veja o arquivo
[LICENSE](LICENSE).
