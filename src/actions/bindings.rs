use gtk::prelude::*;
use libadwaita::prelude::ComboRowExt;

use galaxybook_sound::AudioProfile;

use crate::ui::SoundWindow;

impl SoundWindow {
    pub(crate) fn bind_events(&self) {
        let this = self.clone();
        self.refresh_button.connect_clicked(move |_| {
            this.refresh_state();
        });

        let this = self.clone();
        self.update_button.connect_clicked(move |_| {
            this.install_updates();
        });

        let this = self.clone();
        self.profile_row.connect_selected_notify(move |row| {
            if this.ui_syncing.get() {
                return;
            }
            this.load_profile_into_controls(
                AudioProfile::from_selected_index(row.selected()),
                false,
            );
        });

        let this = self.clone();
        self.atmos_switch_row.connect_active_notify(move |row| {
            if this.ui_syncing.get() {
                return;
            }
            this.set_atmos_enabled(row.is_active(), false);
        });

        let this = self.clone();
        self.combined_output_switch_row
            .connect_active_notify(move |row| {
                if this.ui_syncing.get() {
                    return;
                }
                this.set_combined_output_enabled(row.is_active(), false);
            });

        let this = self.clone();
        self.apply_equalizer_button.connect_clicked(move |_| {
            this.apply_current_equalizer();
        });

        let this = self.clone();
        self.reset_equalizer_button.connect_clicked(move |_| {
            this.restore_selected_profile_curve();
        });

        for (index, control) in self.band_controls.iter().enumerate() {
            let this = self.clone();
            control.scale.connect_value_changed(move |scale| {
                this.update_band_value(index, scale.value());
            });
        }
    }

    pub(crate) fn set_controls_sensitive(&self, sensitive: bool) {
        self.refresh_button.set_sensitive(sensitive);
        self.update_button.set_sensitive(sensitive);
        self.profile_row.set_sensitive(sensitive);
        self.atmos_switch_row.set_sensitive(sensitive);
        self.combined_output_switch_row.set_sensitive(sensitive);
        self.apply_equalizer_button.set_sensitive(sensitive);
        self.reset_equalizer_button.set_sensitive(sensitive);

        for control in &self.band_controls {
            control.scale.set_sensitive(sensitive);
        }
    }

    fn update_band_value(&self, index: usize, value: f64) {
        if let Some(control) = self.band_controls.get(index) {
            control.value_label.set_label(&Self::db_label(value));
        }
        if let Some(band) = self.sound_config.borrow_mut().bands_db.get_mut(index) {
            *band = value;
        }
    }

    pub(crate) fn db_label(value: f64) -> String {
        if value > 0.0 {
            format!("+{value:.1} dB")
        } else {
            format!("{value:.1} dB")
        }
    }
}
