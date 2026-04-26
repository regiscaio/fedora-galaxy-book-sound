use libadwaita as adw;
use libadwaita::prelude::*;

use galaxybook_sound::{APP_NAME, AudioProfile, EQUALIZER_FREQUENCIES_HZ, SoundAppConfig, tr};

use crate::ui::window::EqualizerBandControl;
use crate::ui::{
    InfoRow, build_combo_row, build_linked_buttons_row, build_scrolled_navigation_page,
};

pub(super) struct SoundPage {
    pub(super) page: adw::NavigationPage,
    pub(super) apply_equalizer_button: gtk::Button,
    pub(super) reset_equalizer_button: gtk::Button,
    pub(super) engine_status_row: InfoRow,
    pub(super) active_preset_row: InfoRow,
    pub(super) preset_name_row: InfoRow,
    pub(super) profile_row: adw::ComboRow,
    pub(super) atmos_switch_row: adw::SwitchRow,
    pub(super) combined_output_switch_row: adw::SwitchRow,
    pub(super) band_controls: Vec<EqualizerBandControl>,
}

pub(super) fn build_sound_page(initial_config: &SoundAppConfig) -> SoundPage {
    let engine_status_row = InfoRow::new("Engine DSP");
    let active_preset_row = InfoRow::new("Preset ativo na sessão");
    let preset_name_row = InfoRow::new("Preset local");
    preset_name_row.set_subtitle(&initial_config.preset_name());

    let profile_values = AudioProfile::selectable()
        .iter()
        .map(|profile| tr(profile.title()).to_string())
        .collect::<Vec<_>>();
    let profile_row = build_combo_row(
        &tr("Perfil base"),
        Some(&tr(
            "Escolha a curva usada como ponto de partida para as 10 bandas.",
        )),
        &profile_values,
    );
    profile_row.set_selected(initial_config.selected_profile.selected_index());

    let atmos_switch_row = adw::SwitchRow::builder()
        .title(tr("Atmos compatível"))
        .subtitle(tr(
            "Ativa um pipeline PipeWire com reforço moderado e abertura estéreo, sem DSP proprietário.",
        ))
        .build();
    atmos_switch_row.set_active(initial_config.atmos_enabled);

    let combined_output_switch_row = adw::SwitchRow::builder()
        .title(tr("Saída combinada"))
        .subtitle(tr(
            "Duplica o áudio processado para todos os dispositivos de saída disponíveis.",
        ))
        .build();
    combined_output_switch_row.set_active(initial_config.combined_output_enabled);

    let apply_equalizer_button = gtk::Button::with_label(&tr("Aplicar"));
    apply_equalizer_button.add_css_class("suggested-action");
    let reset_equalizer_button = gtk::Button::with_label(&tr("Restaurar"));

    let actions_group = adw::PreferencesGroup::builder()
        .title(tr("Aplicação"))
        .description(tr(
            "Aplique a curva atual no PipeWire ou restaure as bandas do perfil base selecionado.",
        ))
        .build();
    actions_group.add(&build_linked_buttons_row(&[
        &apply_equalizer_button,
        &reset_equalizer_button,
    ]));

    let session_group = adw::PreferencesGroup::builder()
        .title(tr("Sessão atual"))
        .description(tr(
            "Estado do pipeline do app e do perfil ativo na sessão de áudio.",
        ))
        .build();
    session_group.add(&engine_status_row.row);
    session_group.add(&active_preset_row.row);

    let tuning_group = adw::PreferencesGroup::builder()
        .title(tr("Configuração"))
        .description(tr(
            "Perfil base, modos de processamento e nome do preset local que será salvo.",
        ))
        .build();
    tuning_group.add(&profile_row);
    tuning_group.add(&atmos_switch_row);
    tuning_group.add(&combined_output_switch_row);
    tuning_group.add(&preset_name_row.row);

    let bands_group = adw::PreferencesGroup::builder()
        .title(tr("Equalizador de 10 bandas"))
        .description(tr(
            "Ajuste fino por frequência sobre o perfil base selecionado.",
        ))
        .build();
    let mut band_controls = Vec::with_capacity(EQUALIZER_FREQUENCIES_HZ.len());
    for (index, frequency_hz) in EQUALIZER_FREQUENCIES_HZ.iter().enumerate() {
        let row = adw::ActionRow::builder()
            .title(format!("{frequency_hz} Hz"))
            .subtitle(db_label(initial_config.bands_db[index]))
            .build();
        row.set_activatable(false);

        let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, -12.0, 12.0, 0.1);
        scale.set_draw_value(false);
        scale.set_hexpand(true);
        scale.set_valign(gtk::Align::Center);
        scale.set_width_request(240);
        scale.set_value(initial_config.bands_db[index]);

        let value_label = gtk::Label::new(Some(&db_label(initial_config.bands_db[index])));
        value_label.set_valign(gtk::Align::Center);
        value_label.add_css_class("caption");
        value_label.add_css_class("dim-label");
        value_label.add_css_class("scale-value");

        row.add_suffix(&scale);
        row.add_suffix(&value_label);
        bands_group.add(&row);

        band_controls.push(EqualizerBandControl { scale, value_label });
    }

    let page_content = adw::PreferencesPage::builder()
        .name("sound")
        .title(APP_NAME)
        .build();
    page_content.add(&actions_group);
    page_content.add(&session_group);
    page_content.add(&tuning_group);
    page_content.add(&bands_group);

    SoundPage {
        page: build_scrolled_navigation_page(&page_content, APP_NAME, "sound"),
        apply_equalizer_button,
        reset_equalizer_button,
        engine_status_row,
        active_preset_row,
        preset_name_row,
        profile_row,
        atmos_switch_row,
        combined_output_switch_row,
        band_controls,
    }
}

fn db_label(value: f64) -> String {
    if value > 0.0 {
        format!("+{value:.1} dB")
    } else {
        format!("{value:.1} dB")
    }
}
