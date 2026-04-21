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

## Installazione rapida

Per installare l'app dal repository DNF pubblico:

```bash
sudo dnf config-manager addrepo --from-repofile=https://packages.caioregis.com/fedora/caioregis.repo
sudo dnf install galaxybook-sound
```

Se vuoi anche l'assistente grafico per installazione, validazione e diagnostica
del notebook:

```bash
sudo dnf install galaxybook-setup
```

`Galaxy Book Sound` è un'app audio per Fedora sui notebook Samsung Galaxy
Book, con focus attuale sul **Galaxy Book4 Ultra**. L'app ha una UI nativa
GNOME con `GTK4` e `libadwaita`, ed è stata pensata per lavorare insieme al
supporto per gli altoparlanti pacchettizzato in
[`fedora-galaxy-book-max98390`](https://github.com/regiscaio/fedora-galaxy-book-max98390).

Questo repository copre il **lato userspace** della regolazione audio:
equalizzatore, profili e modalità `Atmos compatibile` per gli altoparlanti
interni. Installazione guidata, validazione dell'ambiente e diagnostica
generale del notebook restano in
[`fedora-galaxy-book-setup`](https://github.com/regiscaio/fedora-galaxy-book-setup).

## Ambito

Il progetto offre:

- profili base `Neutro`, `Musica` e `Cinema`;
- equalizzatore a 10 bande con regolazione manuale;
- modalità `Atmos compatibile` attivabile e disattivabile dall'app;
- interfaccia a pagina singola nel modello delle preferenze GNOME;
- configurazione persistita direttamente dall'app per il flusso audio interno.

Questo progetto **non** offre:

- installazione di driver, moduli o servizi dello stack audio;
- diagnostica hardware o dello stack audio dell'host;
- Dolby Atmos proprietario.

`Galaxy Book Setup` resta il percorso consigliato per installazione guidata e
validazione dell'host. Il supporto MAX98390 resta in
[`fedora-galaxy-book-max98390`](https://github.com/regiscaio/fedora-galaxy-book-max98390).

## Come l'app applica l'audio

Nell'uso quotidiano l'idea è semplice: scegli un profilo, attivi o disattivi
la modalità `Atmos compatibile`, regoli l'equalizzatore e applichi la
configurazione senza uscire dal flusso nativo di GNOME.

Sotto il cofano, l'app mantiene una propria configurazione audio per gli
altoparlanti interni del notebook. In pratica, questo significa:

- equalizzatore, profili e modalità `Atmos compatibile` persistiti
  direttamente dall'app;
- applicazione trasparente sull'uscita interna dopo il riavvio della sessione
  audio;
- separazione chiara tra l'app di uso quotidiano e il resto dello stack di
  sistema.

Tecnicamente, oggi questo avviene tramite un `filter-chain` proprietario in
`~/.config/pipewire/pipewire.conf.d/`, applicato con la policy `smart
filters` di `WirePlumber`.

## Build e packaging

Dipendenze di build su Fedora:

```bash
sudo dnf install cargo rust pkgconf-pkg-config gtk4-devel libadwaita-devel
```

Se l'host non ha disponibile il toolchain completo, il `Makefile` usa un
container rootless con `podman`.

Comandi principali:

```bash
make build
make test
make smoke-test
make dist
make srpm
make rpm
```

Il binario generato localmente si trova in:

```bash
./target/release/galaxybook-sound
```

Il launcher locale di sviluppo può essere installato con:

```bash
make install-local
```

File rilevanti:

- spec RPM: [`packaging/fedora/galaxybook-sound.spec`](packaging/fedora/galaxybook-sound.spec)
- launcher: [`data/com.caioregis.GalaxyBookSound.desktop`](data/com.caioregis.GalaxyBookSound.desktop)
- metadati AppStream: [`data/com.caioregis.GalaxyBookSound.metainfo.xml`](data/com.caioregis.GalaxyBookSound.metainfo.xml)

L'RPM include solo ciò che l'app usa davvero. Il backend audio del progetto
vive dentro l'app stessa tramite un `filter-chain` in `PipeWire`.

## Licenza

Questo progetto è distribuito con licenza **GPL-3.0-only**. Vedi [LICENSE](LICENSE).
