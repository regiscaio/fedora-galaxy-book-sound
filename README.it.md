<p align="center">
  <img src="assets/galaxybook-sound.svg" alt="Icona di Galaxy Book Sound" width="112">
</p>

<h1 align="center">Galaxy Book Sound</h1>

<p align="center">
  <a href="README.md">🇧🇷 Português</a>
  <a href="README.en.md">🇺🇸 English</a>
  <a href="README.es.md">🇪🇸 Español</a>
  <a href="README.it.md">🇮🇹 Italiano</a>
</p>

`Galaxy Book Sound` è un pannello audio nativo GTK4/libadwaita per il Galaxy
Book4 Ultra su Fedora. Lo scopo è volutamente ristretto: equalizzatore a 10
bande, profili pronti e un interruttore `Atmos compatibile` basato su
PipeWire e WirePlumber.

L'interfaccia segue il modello nativo delle preferenze GNOME in una singola
pagina, con selezione del profilo base, interruttore `Atmos compatibile`,
equalizzatore e azioni di applicazione raggruppate.

## Ambito

L'app:

- carica i profili `Neutro`, `Musica` e `Cinema`;
- permette la regolazione fine di 10 bande;
- attiva e disattiva la modalità Atmos compatibile;
- salva e applica la pipeline dell'app alla sessione audio.

L'app non:

- installa driver, moduli o servizi audio;
- esegue diagnostica hardware;
- abilita Dolby Atmos proprietario.

Installazione e diagnostica restano in
[`fedora-galaxy-book-setup`](https://github.com/regiscaio/fedora-galaxy-book-setup).
Il supporto MAX98390 resta in
[`fedora-galaxy-book-max98390`](https://github.com/regiscaio/fedora-galaxy-book-max98390).

## PipeWire

L'app ora scrive un proprio `filter-chain` in
`~/.config/pipewire/pipewire.conf.d/` e lo applica tramite gli `smart filters`
di WirePlumber, puntando al sink interno del notebook.

Questo significa:

- backend gestito direttamente dall'app;
- configurazione persistita direttamente dall'app;
- applicazione trasparente dopo il riavvio della sessione audio.

## Installazione

```bash
sudo dnf config-manager addrepo --from-repofile=https://packages.caioregis.com/fedora/caioregis.repo
sudo dnf install galaxybook-sound
```

Se vuoi il flusso completo del notebook, compreso stack audio e diagnostica,
usa `fedora-galaxy-book-setup`.

## Build

Dipendenze di build su Fedora:

```bash
sudo dnf install cargo rust pkgconf-pkg-config gtk4-devel libadwaita-devel
```

Comandi principali:

```bash
make build
make test
make smoke-test
make rpm
```

## Packaging

File rilevanti:

- spec RPM: [`packaging/fedora/galaxybook-sound.spec`](packaging/fedora/galaxybook-sound.spec)
- launcher: [`data/com.caioregis.GalaxyBookSound.desktop`](data/com.caioregis.GalaxyBookSound.desktop)
- metadati AppStream: [`data/com.caioregis.GalaxyBookSound.metainfo.xml`](data/com.caioregis.GalaxyBookSound.metainfo.xml)

L'RPM include solo ciò che l'app usa davvero, perché il backend ora è un
proprio `filter-chain` in PipeWire.

## Licenza

Questo progetto è distribuito con licenza **GPL-3.0-only**. Vedi [LICENSE](LICENSE).
