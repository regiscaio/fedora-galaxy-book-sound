mod build;
mod pages;
mod shell;

use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

use gtk::gio;
use gtk::prelude::*;
use libadwaita as adw;

use galaxybook_sound::{SoundAppConfig, SoundSessionState};

use crate::ui::InfoRow;

#[derive(Clone)]
pub(crate) struct EqualizerBandControl {
    pub(crate) scale: gtk::Scale,
    pub(crate) value_label: gtk::Label,
}

#[derive(Clone)]
pub(crate) struct SoundWindow {
    pub(crate) window: adw::ApplicationWindow,
    pub(crate) toast_overlay: adw::ToastOverlay,
    pub(crate) update_button: gtk::Button,
    pub(crate) refresh_button: gtk::Button,
    pub(crate) apply_equalizer_button: gtk::Button,
    pub(crate) reset_equalizer_button: gtk::Button,
    pub(crate) engine_status_row: InfoRow,
    pub(crate) active_preset_row: InfoRow,
    pub(crate) preset_name_row: InfoRow,
    pub(crate) profile_row: adw::ComboRow,
    pub(crate) atmos_switch_row: adw::SwitchRow,
    pub(crate) combined_output_switch_row: adw::SwitchRow,
    pub(crate) band_controls: Vec<EqualizerBandControl>,
    pub(crate) sound_config: Rc<RefCell<SoundAppConfig>>,
    pub(crate) session_state: Rc<RefCell<Option<SoundSessionState>>>,
    pub(crate) ui_syncing: Rc<Cell<bool>>,
}

impl SoundWindow {
    pub(crate) fn present(&self) {
        self.window.present();
        self.refresh_updates();
        self.refresh_state();
    }

    fn install_actions(&self, app: &adw::Application) {
        let action = gio::SimpleAction::new("about", None);
        let this = self.clone();
        action.connect_activate(move |_, _| {
            this.present_about_dialog();
        });
        app.add_action(&action);
    }
}
