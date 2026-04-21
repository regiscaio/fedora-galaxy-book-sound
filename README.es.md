<p align="center">
  <img src="assets/galaxybook-sound.svg" alt="Icono de Galaxy Book Sound" width="112">
</p>

<h1 align="center">Galaxy Book Sound</h1>

<p align="center">
  <a href="README.md">🇧🇷 Português</a>
  <a href="README.en.md">🇺🇸 English</a>
  <a href="README.es.md">🇪🇸 Español</a>
  <a href="README.it.md">🇮🇹 Italiano</a>
</p>

`Galaxy Book Sound` es una aplicación nativa GTK4/libadwaita para ajustar el
sonido del Galaxy Book4 Ultra en Fedora. Su alcance es simple: ecualizador de
10 bandas, perfiles listos y un interruptor de `Atmos compatible` basado en
PipeWire y WirePlumber.

La interfaz sigue el patrón nativo de preferencias de GNOME en una sola página,
con selección del perfil base, interruptor de `Atmos compatible`, ecualizador y
acciones de aplicación agrupadas.

## Alcance

La aplicación:

- carga perfiles `Neutro`, `Música` y `Cinema`;
- permite ajustar 10 bandas;
- activa y desactiva el modo Atmos compatible;
- guarda y aplica el pipeline del app en la sesión de audio.

La aplicación no:

- instala drivers, módulos o servicios del stack de audio;
- ejecuta diagnósticos de hardware;
- activa Dolby Atmos propietario.

La instalación y los diagnósticos pertenecen a
[`fedora-galaxy-book-setup`](https://github.com/regiscaio/fedora-galaxy-book-setup).
El soporte MAX98390 permanece en
[`fedora-galaxy-book-max98390`](https://github.com/regiscaio/fedora-galaxy-book-max98390).

## PipeWire

La app ahora escribe su propio `filter-chain` en
`~/.config/pipewire/pipewire.conf.d/` y lo aplica con la política de `smart
filters` de WirePlumber, apuntando al sink interno del notebook.

Eso implica:

- backend mantenido por la propia app;
- configuración persistida por la propia app;
- aplicación transparente tras reiniciar la sesión de audio.

## Instalación

```bash
sudo dnf config-manager addrepo --from-repofile=https://packages.caioregis.com/fedora/caioregis.repo
sudo dnf install galaxybook-sound
```

Si quieres el flujo completo del portátil, incluido el stack de audio y los
diagnósticos, usa `fedora-galaxy-book-setup`.

## Build

Dependencias de build en Fedora:

```bash
sudo dnf install cargo rust pkgconf-pkg-config gtk4-devel libadwaita-devel
```

Comandos principales:

```bash
make build
make test
make smoke-test
make rpm
```

## Empaquetado

Archivos relevantes:

- spec RPM: [`packaging/fedora/galaxybook-sound.spec`](packaging/fedora/galaxybook-sound.spec)
- launcher: [`data/com.caioregis.GalaxyBookSound.desktop`](data/com.caioregis.GalaxyBookSound.desktop)
- metadatos AppStream: [`data/com.caioregis.GalaxyBookSound.metainfo.xml`](data/com.caioregis.GalaxyBookSound.metainfo.xml)

El RPM solo incluye lo que la app realmente usa, porque el backend ahora es su
propio `filter-chain` en PipeWire.

## Licencia

Este proyecto se distribuye bajo **GPL-3.0-only**. Consulta [LICENSE](LICENSE).
