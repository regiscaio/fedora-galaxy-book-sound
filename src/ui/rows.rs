use libadwaita as adw;
use libadwaita::prelude::*;

use galaxybook_sound::tr;

#[derive(Clone)]
pub(crate) struct InfoRow {
    pub(crate) row: adw::ActionRow,
}

impl InfoRow {
    pub(crate) fn new(title: &'static str) -> Self {
        let row = adw::ActionRow::builder().title(tr(title)).build();
        row.set_subtitle(&tr("Coletando…"));
        Self { row }
    }

    pub(crate) fn set_subtitle(&self, subtitle: &str) {
        self.row.set_subtitle(subtitle);
    }
}
