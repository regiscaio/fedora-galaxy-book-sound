use std::sync::mpsc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use galaxybook_sound::{
    AudioProfile, SoundSessionState, collect_sound_session_state, default_bands_for_profile,
    save_and_apply_profile, tr, trf,
};

use crate::ui::SoundWindow;

impl SoundWindow {
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
        self.set_controls_sensitive(false);
        self.engine_status_row
            .set_subtitle(&tr("Aplicando a nova curva e reiniciando a sessão de áudio…"));

        let result = save_and_apply_profile(&self.sound_config.borrow().clone());

        match result {
            Ok(preset_name) => {
                self.update_equalizer_summary();
                self.toast_overlay.add_toast(adw::Toast::new(&trf(
                    "Perfil aplicado no PipeWire: {preset}",
                    &[("preset", preset_name)],
                )));
                self.refresh_state();
            }
            Err(error) => {
                self.refresh_button.set_sensitive(true);
                self.set_controls_sensitive(true);
                self.present_command_result_dialog(&tr("Aplicar"), &error);
            }
        }
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
