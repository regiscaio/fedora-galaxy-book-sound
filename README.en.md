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

## Quick install

To install the app from the public DNF repository:

```bash
sudo dnf config-manager addrepo --from-repofile=https://packages.caioregis.com/fedora/caioregis.repo
sudo dnf install galaxybook-sound
```

If you also want the notebook's graphical setup, validation, and diagnostics
helper:

```bash
sudo dnf install galaxybook-setup
```

`Galaxy Book Sound` is a sound app for Fedora on Samsung Galaxy Book laptops,
with current focus on the **Galaxy Book4 Ultra**. It has a native GNOME UI
built with `GTK4` and `libadwaita`, and it was designed to work alongside the
speaker support packaged in
[`fedora-galaxy-book-max98390`](https://github.com/regiscaio/fedora-galaxy-book-max98390).

This repository covers the **userspace side** of audio adjustment: EQ,
profiles, and the `Atmos compatible` mode for the internal speakers. Guided
installation, environment validation, and broader notebook diagnostics live in
[`fedora-galaxy-book-setup`](https://github.com/regiscaio/fedora-galaxy-book-setup).

## Scope

The project delivers:

- `Flat`, `Music`, and `Cinema` base profiles;
- a 10-band equalizer with manual adjustment;
- an `Atmos compatible` mode that can be enabled and disabled from the app;
- a single-page UI following the GNOME preferences pattern;
- app-owned persisted configuration for the internal audio flow.

This project does **not** provide:

- installation of audio-stack drivers, modules, or services;
- hardware or host audio-stack diagnostics;
- proprietary Dolby Atmos.

`Galaxy Book Setup` remains the recommended path for guided installation and
host validation. MAX98390 support stays in
[`fedora-galaxy-book-max98390`](https://github.com/regiscaio/fedora-galaxy-book-max98390).

## How the app applies sound

In day-to-day use, the idea is simple: pick a profile, turn `Atmos compatible`
mode on or off, fine-tune the equalizer, and apply the configuration without
leaving the native GNOME flow.

Under the hood, the app keeps its own audio configuration for the notebook's
internal speakers. In practice, that means:

- EQ, profiles, and `Atmos compatible` mode persisted by the app itself;
- transparent application to the internal output after restarting the audio
  session;
- clear separation between the daily-use app and the rest of the system stack.

Technically, this is currently done with an app-owned `filter-chain` under
`~/.config/pipewire/pipewire.conf.d/`, applied through WirePlumber `smart
filters`.

## Build and packaging

Build dependencies on Fedora:

```bash
sudo dnf install cargo rust pkgconf-pkg-config gtk4-devel libadwaita-devel
```

If the host does not have the full toolchain available, the `Makefile` falls
back to a rootless `podman` container.

Main commands:

```bash
make build
make test
make smoke-test
make dist
make srpm
make rpm
```

The locally built binary ends up at:

```bash
./target/release/galaxybook-sound
```

The local development launcher can be installed with:

```bash
make install-local
```

Relevant files:

- RPM spec: [`packaging/fedora/galaxybook-sound.spec`](packaging/fedora/galaxybook-sound.spec)
- launcher: [`data/com.caioregis.GalaxyBookSound.desktop`](data/com.caioregis.GalaxyBookSound.desktop)
- AppStream metadata: [`data/com.caioregis.GalaxyBookSound.metainfo.xml`](data/com.caioregis.GalaxyBookSound.metainfo.xml)

The RPM only carries what the app actually uses. The project's audio backend
lives inside the app itself through a PipeWire `filter-chain`.

## License

This project is distributed under **GPL-3.0-only**. See [LICENSE](LICENSE).
