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

## Instalación rápida

Para instalar la app desde el repositorio DNF público:

```bash
sudo dnf config-manager addrepo --from-repofile=https://packages.caioregis.com/fedora/caioregis.repo
sudo dnf install galaxybook-sound
```

Si también quieres el asistente gráfico de instalación, validación y
diagnóstico del portátil:

```bash
sudo dnf install galaxybook-setup
```

`Galaxy Book Sound` es una app de sonido para Fedora en portátiles Samsung
Galaxy Book, con foco actual en el **Galaxy Book4 Ultra**. La app tiene UI
nativa de GNOME con `GTK4` y `libadwaita`, y fue pensada para trabajar junto
con el soporte de altavoces empaquetado en
[`fedora-galaxy-book-max98390`](https://github.com/regiscaio/fedora-galaxy-book-max98390).

Este repositorio cubre el **lado userspace** del ajuste de audio:
ecualizador, perfiles y el modo `Atmos compatible` para los altavoces
internos. La instalación guiada, la validación del entorno y los diagnósticos
generales del portátil quedan en
[`fedora-galaxy-book-setup`](https://github.com/regiscaio/fedora-galaxy-book-setup).

## Interfaz actual

### Pantalla principal

![Galaxy Book Sound — pantalla principal](img/app-sound-galaxy-1.png)

### Ecualizador de 10 bandas

![Galaxy Book Sound — ecualizador de 10 bandas](img/app-sound-galaxy-2.png)

### Modal `Sobre`

![Galaxy Book Sound — Sobre](img/app-sound-galaxy-3.png)

## Alcance

El proyecto ofrece:

- perfiles base `Neutro`, `Música` y `Cinema`;
- ecualizador de 10 bandas con ajuste manual;
- modo `Atmos compatible` que se puede activar y desactivar desde la app;
- interfaz de una sola página siguiendo el patrón de preferencias de GNOME;
- configuración persistida por la propia app para el flujo de audio interno.

Este proyecto **no** ofrece:

- instalación de drivers, módulos o servicios del stack de audio;
- diagnóstico de hardware o del stack de audio del host;
- Dolby Atmos propietario.

`Galaxy Book Setup` sigue siendo el camino recomendado para la instalación
guiada y la validación del host. El soporte MAX98390 permanece en
[`fedora-galaxy-book-max98390`](https://github.com/regiscaio/fedora-galaxy-book-max98390).

## Cómo la app aplica el sonido

En el uso diario, la idea es simple: eliges un perfil, activas o desactivas el
modo `Atmos compatible`, ajustas el ecualizador y aplicas la configuración sin
salir del flujo nativo de GNOME.

Por debajo, la app mantiene su propia configuración de audio para los
altavoces internos del portátil. En la práctica, eso significa:

- ecualizador, perfiles y modo `Atmos compatible` persistidos por la propia
  app;
- aplicación transparente en la salida interna después de reiniciar la sesión
  de audio;
- separación clara entre la app de uso diario y el resto del stack del sistema.

Técnicamente, esto hoy se hace con un `filter-chain` propio en
`~/.config/pipewire/pipewire.conf.d/`, aplicado mediante la política de `smart
filters` de `WirePlumber`.

## Build y empaquetado

Dependencias de build en Fedora:

```bash
sudo dnf install cargo rust pkgconf-pkg-config gtk4-devel libadwaita-devel
```

Si el host no tiene disponible el toolchain completo, el `Makefile` usa un
contenedor rootless con `podman`.

Comandos principales:

```bash
make build
make test
make smoke-test
make dist
make srpm
make rpm
```

El binario generado localmente queda en:

```bash
./target/release/galaxybook-sound
```

El launcher local de desarrollo puede instalarse con:

```bash
make install-local
```

Archivos relevantes:

- spec RPM: [`packaging/fedora/galaxybook-sound.spec`](packaging/fedora/galaxybook-sound.spec)
- launcher: [`data/com.caioregis.GalaxyBookSound.desktop`](data/com.caioregis.GalaxyBookSound.desktop)
- metadatos AppStream: [`data/com.caioregis.GalaxyBookSound.metainfo.xml`](data/com.caioregis.GalaxyBookSound.metainfo.xml)

El RPM solo incluye lo que la app realmente usa. El backend de audio del
proyecto vive dentro de la propia app mediante un `filter-chain` en `PipeWire`.

## Licencia

Este proyecto se distribuye bajo **GPL-3.0-only**. Consulta [LICENSE](LICENSE).
