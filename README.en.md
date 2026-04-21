<p align="center">
  <img src="assets/galaxybook-sound.svg" alt="Galaxy Book Sound icon" width="112">
</p>

<h1 align="center">Galaxy Book Sound</h1>

<p align="center">
  <a href="README.md">🇧🇷 Português</a>
  <a href="README.en.md">🇺🇸 English</a>
  <a href="README.es.md">🇪🇸 Español</a>
  <a href="README.it.md">🇮🇹 Italiano</a>
</p>

`Galaxy Book Sound` is a native GTK4/libadwaita sound panel for Fedora on the
Galaxy Book4 Ultra. Its scope is intentionally narrow: a 10-band equalizer,
ready-made profiles, and an `Atmos compatible` toggle powered directly by
PipeWire and WirePlumber.

The UI follows the native GNOME preferences pattern in a single page, with base
profile selection, an `Atmos compatible` toggle, the equalizer, and grouped
apply actions.

## Scope

This app does:

- load `Flat`, `Music`, and `Cinema` base profiles;
- let you fine-tune 10 EQ bands;
- turn the compatible Atmos-like mode on and off;
- save and apply the app pipeline to the live session.

This app does not:

- install drivers, modules, or audio services;
- run hardware diagnostics;
- enable proprietary Dolby Atmos.

Installation and diagnostics belong in
[`fedora-galaxy-book-setup`](https://github.com/regiscaio/fedora-galaxy-book-setup).
The MAX98390 support stays in
[`fedora-galaxy-book-max98390`](https://github.com/regiscaio/fedora-galaxy-book-max98390).

## PipeWire

The app now writes its own `filter-chain` drop-in under
`~/.config/pipewire/pipewire.conf.d/` and applies it through WirePlumber smart
filters, targeting the notebook's internal speaker sink.

That means:

- an app-owned backend;
- app-owned persisted configuration;
- transparent application after the audio session restart.

## Install

```bash
sudo dnf config-manager addrepo --from-repofile=https://packages.caioregis.com/fedora/caioregis.repo
sudo dnf install galaxybook-sound
```

For the full notebook flow, including audio-stack setup and diagnostics, use
`fedora-galaxy-book-setup`.

## Build

Build dependencies on Fedora:

```bash
sudo dnf install cargo rust pkgconf-pkg-config gtk4-devel libadwaita-devel
```

Main commands:

```bash
make build
make test
make smoke-test
make rpm
```

## Packaging

Relevant files:

- RPM spec: [`packaging/fedora/galaxybook-sound.spec`](packaging/fedora/galaxybook-sound.spec)
- launcher: [`data/com.caioregis.GalaxyBookSound.desktop`](data/com.caioregis.GalaxyBookSound.desktop)
- AppStream metadata: [`data/com.caioregis.GalaxyBookSound.metainfo.xml`](data/com.caioregis.GalaxyBookSound.metainfo.xml)

The RPM only carries what the app actually uses, because the backend is now the
app's own PipeWire `filter-chain`.

## License

This project is distributed under **GPL-3.0-only**. See [LICENSE](LICENSE).
