mod actions;
mod ui;

use gtk::glib;
use gtk::prelude::*;
use libadwaita as adw;

use galaxybook_sound::{APP_ID, init_i18n, run_smoke_test};
use ui::SoundWindow;

fn main() -> glib::ExitCode {
    init_i18n();
    if std::env::args().any(|arg| arg == "--smoke-test") {
        return match run_smoke_test() {
            Ok(()) => glib::ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                glib::ExitCode::FAILURE
            }
        };
    }

    adw::init().expect("Failed to initialize libadwaita");

    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| {
        if let Some(window) = app.active_window() {
            window.present();
            return;
        }

        let window = SoundWindow::new(app);
        window.present();
    });

    app.run()
}
