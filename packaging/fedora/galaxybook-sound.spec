%global app_id com.caioregis.GalaxyBookSound
%global pkg_version %{?pkg_version_override}%{!?pkg_version_override:1.0.0}
%global source_date_epoch_from_changelog 0
%global clamp_mtime_to_source_date_epoch 1

Name:           galaxybook-sound
Version:        %{pkg_version}
Release:        1%{?dist}
Summary:        Native PipeWire equalizer and sound profiles for Galaxy Book on Fedora

License:        GPL-3.0-only
URL:            https://github.com/regiscaio/fedora-galaxy-book-sound
Source0:        %{name}-%{version}.tar.gz

ExclusiveArch:  x86_64

BuildRequires:  cargo
BuildRequires:  clang
BuildRequires:  desktop-file-utils
BuildRequires:  gettext
BuildRequires:  gcc-c++
BuildRequires:  make
BuildRequires:  pkgconfig(gtk4)
BuildRequires:  pkgconfig(libadwaita-1)
BuildRequires:  rust

Requires:       pipewire
Requires:       polkit
Requires:       systemd
Requires:       wireplumber

%description
Galaxy Book Sound is a native GTK4 and libadwaita helper for Fedora on Galaxy
Book notebooks. It focuses on sound tuning through a native PipeWire
filter-chain, 10-band equalization, ready-made profiles, and an
Atmos-compatible toggle.

%prep
%autosetup -n %{name}-%{version}

%build
APP_VERSION_OVERRIDE=%{version} cargo --offline build --release --locked --bin galaxybook-sound

%install
install -Dm755 target/release/galaxybook-sound %{buildroot}%{_bindir}/galaxybook-sound
install -Dm644 assets/galaxybook-sound.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/%{app_id}.svg
for lang in en es it; do \
  install -d %{buildroot}%{_datadir}/locale/${lang}/LC_MESSAGES; \
  msgfmt po/${lang}.po -o %{buildroot}%{_datadir}/locale/${lang}/LC_MESSAGES/%{name}.mo; \
done
sed \
  -e 's|@EXEC@|galaxybook-sound|g' \
  -e 's|@ICON@|%{app_id}|g' \
  -e 's|@STARTUP_WM_CLASS@|%{app_id}|g' \
  data/%{app_id}.desktop > %{app_id}.desktop
install -Dm644 %{app_id}.desktop %{buildroot}%{_datadir}/applications/%{app_id}.desktop
install -Dm644 data/%{app_id}.metainfo.xml %{buildroot}%{_datadir}/metainfo/%{app_id}.metainfo.xml

%check
desktop-file-validate %{app_id}.desktop
APP_VERSION_OVERRIDE=%{version} cargo --offline test --locked --lib --bin galaxybook-sound

%files
%license LICENSE
%{_bindir}/galaxybook-sound
%{_datadir}/applications/%{app_id}.desktop
%{_datadir}/icons/hicolor/scalable/apps/%{app_id}.svg
%{_datadir}/locale/en/LC_MESSAGES/%{name}.mo
%{_datadir}/locale/es/LC_MESSAGES/%{name}.mo
%{_datadir}/locale/it/LC_MESSAGES/%{name}.mo
%{_datadir}/metainfo/%{app_id}.metainfo.xml

%changelog
