use std::cell::{Cell, RefCell};
use std::rc::Rc;

use libadwaita as adw;

use galaxybook_sound::load_sound_app_config;

use crate::ui::install_css;

use super::SoundWindow;
use super::pages::build_sound_page;
use super::shell::build_window_shell;

impl SoundWindow {
    pub(crate) fn new(app: &adw::Application) -> Self {
        install_css();

        let shell = build_window_shell(app);
        let window = shell.window;
        let toast_overlay = shell.toast_overlay;
        let navigation_view = shell.navigation_view;
        let update_button = shell.update_button;
        let refresh_button = shell.refresh_button;

        let initial_config = load_sound_app_config();
        let sound_config = Rc::new(RefCell::new(initial_config.clone()));
        let session_state = Rc::new(RefCell::new(None));
        let ui_syncing = Rc::new(Cell::new(false));

        let sound_page = build_sound_page(&initial_config);
        navigation_view.add(&sound_page.page);
        navigation_view.replace_with_tags(&["sound"]);

        let instance = Self {
            window,
            toast_overlay,
            update_button,
            refresh_button,
            apply_equalizer_button: sound_page.apply_equalizer_button,
            reset_equalizer_button: sound_page.reset_equalizer_button,
            engine_status_row: sound_page.engine_status_row,
            active_preset_row: sound_page.active_preset_row,
            preset_name_row: sound_page.preset_name_row,
            profile_row: sound_page.profile_row,
            atmos_switch_row: sound_page.atmos_switch_row,
            combined_output_switch_row: sound_page.combined_output_switch_row,
            band_controls: sound_page.band_controls,
            sound_config,
            session_state,
            ui_syncing,
        };

        instance.install_actions(app);
        instance.bind_events();
        instance.update_equalizer_summary();
        instance
    }
}
