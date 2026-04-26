use std::sync::mpsc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use galaxybook_sound::{
    AudioProfile, SoundSessionState, collect_sound_session_state, default_bands_for_profile,
    install_package_updates, package_update_names, save_and_apply_profile, tr, trf,
};

use crate::ui::SoundWindow;

const SOUND_UPDATE_PACKAGES: &[&str] = &[
    "galaxybook-sound",
    "galaxybook-max98390-kmod-common",
    "akmod-galaxybook-max98390",
];

fn update_button_tooltip(packages: &[String]) -> String {
    trf(
        "Baixar e instalar atualizações: {packages}",
        &[("packages", packages.join(", "))],
    )
}

impl SoundWindow {
    pub(crate) fn refresh_updates(&self) {
        self.update_button.set_visible(false);
        self.update_button.set_sensitive(false);

        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(package_update_names(SOUND_UPDATE_PACKAGES));
        });

        let this = self.clone();
        glib::timeout_add_local(Duration::from_millis(150), move || match receiver.try_recv() {
            Ok(Ok(packages)) => {
                let has_updates = !packages.is_empty();
                this.update_button.set_visible(has_updates);
                this.update_button.set_sensitive(has_updates);
                if has_updates {
                    this.update_button
                        .set_tooltip_text(Some(&update_button_tooltip(&packages)));
                }
                glib::ControlFlow::Break
            }
            Ok(Err(_error)) => {
                this.update_button.set_visible(false);
                this.update_button.set_sensitive(false);
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                this.update_button.set_visible(false);
                this.update_button.set_sensitive(false);
                glib::ControlFlow::Break
            }
        });
    }

    pub(crate) fn install_updates(&self) {
        if !self.update_button.is_visible() || !self.update_button.is_sensitive() {
            return;
        }

        self.update_button.set_sensitive(false);
        self.toast_overlay.add_toast(adw::Toast::new(&tr(
            "Baixando e instalando atualizações do Galaxy Book Sound…",
        )));

        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(install_package_updates(SOUND_UPDATE_PACKAGES));
        });

        let this = self.clone();
        glib::timeout_add_local(Duration::from_millis(150), move || match receiver.try_recv() {
            Ok(Ok(_output)) => {
                this.update_button.set_visible(false);
                this.toast_overlay.add_toast(adw::Toast::new(&tr(
                    "Atualizações instaladas. Reinicie o app se ele tiver sido atualizado.",
                )));
                this.refresh_updates();
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                this.update_button.set_sensitive(true);
                this.present_command_result_dialog(&tr("Atualizar pacotes"), &error);
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                this.update_button.set_sensitive(true);
                this.toast_overlay.add_toast(adw::Toast::new(&tr(
                    "Falha ao acompanhar a atualização solicitada.",
                )));
                glib::ControlFlow::Break
            }
        });
    }

    pub(crate) fn refresh_state(&self) {
        self.refresh_button.set_sensitive(false);
        self.set_controls_sensitive(false);
        self.engine_status_row
            .set_subtitle(&tr("Lendo o estado atual do pipeline PipeWire…"));
        self.active_preset_row
            .set_subtitle(&tr("Verificando o perfil ativo da sessão…"));

        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let state = collect_sound_session_state();
            let _ = sender.send(state);
        });

        let this = self.clone();
        glib::timeout_add_local(Duration::from_millis(75), move || match receiver.try_recv() {
            Ok(state) => {
                this.apply_session_state(state);
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                this.refresh_button.set_sensitive(true);
                this.set_controls_sensitive(true);
                this.toast_overlay
                    .add_toast(adw::Toast::new(&tr("Falha ao atualizar o estado do áudio.")));
                glib::ControlFlow::Break
            }
        });
    }

    fn apply_session_state(&self, state: SoundSessionState) {
        let engine_status = if !state.pipewire_available {
            tr("PipeWire ou WirePlumber não responderam nesta sessão.")
        } else if state.filter_active {
            match state.target_output.as_deref() {
                Some(output) => trf(
                    "Pipeline nativo do app ativo e encadeado ao dispositivo: {output}",
                    &[("output", output.to_string())],
                ),
                None => tr("Pipeline nativo do app ativo na sessão atual."),
            }
        } else if state.config_present {
            tr("Configuração local encontrada, mas o pipeline ainda não foi carregado na sessão atual.")
        } else {
            tr("Nenhum pipeline do app foi aplicado ainda.")
        };
        self.engine_status_row.set_subtitle(&engine_status);

        let active_preset = match state.active_profile_name.as_ref() {
            Some(name) => trf("Perfil ativo: {preset}", &[("preset", name.to_string())]),
            None if state.config_present => {
                tr("Há uma configuração local salva, mas ela ainda não está ativa na sessão.")
            }
            None => tr("Nenhum perfil ativo agora."),
        };
        self.active_preset_row.set_subtitle(&active_preset);

        *self.session_state.borrow_mut() = Some(state);
        self.refresh_button.set_sensitive(true);
        self.set_controls_sensitive(true);
    }

    pub(crate) fn load_profile_into_controls(&self, profile: AudioProfile, apply_after: bool) {
        let selected_profile = match profile {
            AudioProfile::AtmosCompatible => AudioProfile::Cinema,
            other => other,
        };
        let bands = default_bands_for_profile(selected_profile);
        {
            let mut config = self.sound_config.borrow_mut();
            config.selected_profile = selected_profile;
            config.bands_db = bands;
        }

        for (index, control) in self.band_controls.iter().enumerate() {
            control.scale.set_value(bands[index]);
            control.value_label.set_label(&Self::db_label(bands[index]));
        }

        self.update_equalizer_summary();

        if apply_after {
            self.apply_current_equalizer();
        }
    }

    pub(crate) fn set_atmos_enabled(&self, enabled: bool, apply_after: bool) {
        let changed = {
            let mut config = self.sound_config.borrow_mut();
            let changed = config.atmos_enabled != enabled;
            config.atmos_enabled = enabled;
            changed
        };

        self.update_equalizer_summary();

        if changed && apply_after {
            self.apply_current_equalizer();
        }
    }

    pub(crate) fn restore_selected_profile_curve(&self) {
        let profile = self.sound_config.borrow().selected_profile;
        self.load_profile_into_controls(profile, false);
        self.toast_overlay.add_toast(adw::Toast::new(&tr(
            "Bandas restauradas para a curva padrão do perfil selecionado.",
        )));
    }

    pub(crate) fn apply_current_equalizer(&self) {
        let config = self.sound_config.borrow().clone();

        self.refresh_button.set_sensitive(false);
        self.set_controls_sensitive(false);
        self.engine_status_row
            .set_subtitle(&tr("Aplicando a nova curva e reiniciando a sessão de áudio…"));

        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(save_and_apply_profile(&config));
        });

        let this = self.clone();
        glib::timeout_add_local(Duration::from_millis(75), move || match receiver.try_recv() {
            Ok(Ok(preset_name)) => {
                this.update_equalizer_summary();
                this.toast_overlay.add_toast(adw::Toast::new(&trf(
                    "Perfil aplicado no PipeWire: {preset}",
                    &[("preset", preset_name)],
                )));
                this.refresh_state();
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                this.refresh_button.set_sensitive(true);
                this.set_controls_sensitive(true);
                this.present_command_result_dialog(&tr("Aplicar"), &error);
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                this.refresh_button.set_sensitive(true);
                this.set_controls_sensitive(true);
                this.toast_overlay.add_toast(adw::Toast::new(&tr(
                    "Falha ao acompanhar a aplicação do perfil de áudio.",
                )));
                glib::ControlFlow::Break
            }
        });
    }

    pub(crate) fn update_equalizer_summary(&self) {
        let config = self.sound_config.borrow();
        let preset_name = config.preset_name();
        let selected_index = config.selected_profile.selected_index();
        let atmos_enabled = config.atmos_enabled;
        drop(config);

        self.ui_syncing.set(true);
        if self.profile_row.selected() != selected_index {
            self.profile_row.set_selected(selected_index);
        }
        if self.atmos_switch_row.is_active() != atmos_enabled {
            self.atmos_switch_row.set_active(atmos_enabled);
        }
        self.ui_syncing.set(false);

        self.preset_name_row.set_subtitle(&preset_name);
    }

    fn present_command_result_dialog(&self, title: &str, output: &str) {
        let dialog = adw::Dialog::builder()
            .title(title)
            .content_width(680)
            .content_height(420)
            .build();

        let header = adw::HeaderBar::new();
        let window_title = adw::WindowTitle::new(title, &tr("Detalhes"));
        header.set_title_widget(Some(&window_title));

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);

        let text_view = gtk::TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .monospace(true)
            .wrap_mode(gtk::WrapMode::WordChar)
            .top_margin(16)
            .bottom_margin(16)
            .left_margin(16)
            .right_margin(16)
            .build();
        let fallback_output = tr("A operação falhou, mas não retornou saída textual.");
        let output_text = if output.trim().is_empty() {
            fallback_output.as_str()
        } else {
            output
        };
        text_view.buffer().set_text(output_text);

        let scroller = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&text_view)
            .build();

        toolbar.set_content(Some(&scroller));
        dialog.set_child(Some(&toolbar));
        dialog.present(Some(&self.window));
    }
}
