pub(crate) mod about;
pub(crate) mod rows;
pub(crate) mod window;

use std::rc::Rc;

use galaxybook_sound::{APP_ID, APP_NAME, tr, tr_mark, trf};
use gtk::glib;
use gtk::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

pub(crate) use self::rows::InfoRow;
pub(crate) use self::window::SoundWindow;

pub(crate) fn build_scrolled_navigation_page(
    page: &adw::PreferencesPage,
    title: &str,
    tag: &str,
) -> adw::NavigationPage {
    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .min_content_width(0)
        .child(page)
        .build();

    adw::NavigationPage::builder()
        .title(title)
        .tag(tag)
        .child(&scroller)
        .can_pop(true)
        .build()
}

pub(crate) fn build_combo_row(
    title: &str,
    subtitle: Option<&str>,
    values: &[String],
) -> adw::ComboRow {
    let string_list = gtk::StringList::new(&[]);
    for value in values {
        string_list.append(value);
    }

    let row = adw::ComboRow::builder().title(title).build();
    if let Some(subtitle) = subtitle {
        row.set_subtitle(subtitle);
    }
    row.set_model(Some(&string_list));
    row
}

pub(crate) fn build_linked_buttons_row(buttons: &[&gtk::Button]) -> gtk::ListBoxRow {
    let action_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    action_box.add_css_class("linked");
    action_box.set_hexpand(true);
    action_box.set_halign(gtk::Align::Fill);
    action_box.set_homogeneous(true);

    for button in buttons {
        button.set_hexpand(true);
        action_box.append(*button);
    }

    let row = gtk::ListBoxRow::new();
    row.set_activatable(false);
    row.set_selectable(false);

    let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    row_box.set_margin_top(6);
    row_box.set_margin_bottom(6);
    row_box.set_margin_start(6);
    row_box.set_margin_end(6);
    row_box.append(&action_box);
    row.set_child(Some(&row_box));

    row
}

pub(crate) fn build_suffix_action_row<F>(
    title: &str,
    subtitle: &str,
    icon_name: &str,
    tooltip: &str,
    on_activate: F,
) -> adw::ActionRow
where
    F: Fn() + 'static,
{
    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(subtitle)
        .build();
    row.set_subtitle_selectable(true);

    let button = gtk::Button::builder()
        .icon_name(icon_name)
        .tooltip_text(tooltip)
        .valign(gtk::Align::Center)
        .build();
    button.add_css_class("flat");

    let callback = Rc::new(on_activate);
    {
        let callback = callback.clone();
        button.connect_clicked(move |_| {
            callback();
        });
    }

    row.add_suffix(&button);
    row.set_activatable_widget(Some(&button));
    row.set_activatable(true);
    row
}

pub(crate) fn build_about_summary_row(
    app_name: &str,
    description: &str,
) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_activatable(false);
    row.set_selectable(false);

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);

    let app_icon = gtk::Image::from_icon_name(APP_ID);
    app_icon.set_pixel_size(48);
    app_icon.set_valign(gtk::Align::Start);

    let text_column = gtk::Box::new(gtk::Orientation::Vertical, 4);
    text_column.set_hexpand(true);
    text_column.set_valign(gtk::Align::Center);

    let title_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    title_row.set_halign(gtk::Align::Start);

    let title_label = gtk::Label::new(None);
    title_label.set_markup(&format!(
        "<span size='large' weight='600'>{}</span>",
        glib::markup_escape_text(app_name)
    ));
    title_label.set_xalign(0.0);

    let version_label = gtk::Label::new(None);
    version_label.set_markup(&format!(
        "<span alpha='55%' size='small'>{}</span>",
        glib::markup_escape_text(
            &trf("Versão {version}", &[("version", env!("APP_VERSION").to_string())]),
        )
    ));
    version_label.set_xalign(0.0);

    let description_label = gtk::Label::new(None);
    description_label.set_markup(&format!(
        "<span alpha='55%' size='small'>{}</span>",
        glib::markup_escape_text(description)
    ));
    description_label.set_xalign(0.0);
    description_label.set_wrap(true);

    title_row.append(&title_label);
    title_row.append(&version_label);
    text_column.append(&title_row);
    text_column.append(&description_label);

    content.append(&app_icon);
    content.append(&text_column);
    row.set_child(Some(&content));
    row
}

pub(crate) fn build_about_details_subpage() -> adw::NavigationPage {
    let page = adw::PreferencesPage::builder()
        .name("details")
        .title(tr("Detalhes"))
        .build();

    let app_group = adw::PreferencesGroup::builder()
        .title(tr("Aplicativo"))
        .description(tr("Identificação pública e técnica do Galaxy Book Sound."))
        .build();

    for (title, subtitle) in [
        (tr_mark("Nome"), APP_NAME.to_string()),
        (tr_mark("Versão"), env!("APP_VERSION").to_string()),
        (tr_mark("App ID"), APP_ID.to_string()),
        (tr_mark("Desktop ID"), format!("{APP_ID}.desktop")),
    ] {
        let row = adw::ActionRow::builder()
            .title(tr(title))
            .subtitle(subtitle)
            .build();
        row.set_activatable(false);
        row.set_subtitle_selectable(true);
        app_group.add(&row);
    }

    let scope_group = adw::PreferencesGroup::builder()
        .title(tr("Escopo atual"))
        .description(tr("Resumo do que o painel de som cobre hoje."))
        .build();
    for (title, subtitle) in [
        (
            tr_mark("Objetivo"),
            tr("Ajudar você a deixar o áudio do Galaxy Book mais agradável no uso diário, com perfis prontos e espaço para ajuste fino quando fizer sentido."),
        ),
        (
            tr_mark("Módulo disponível"),
            tr("Perfis locais, equalizador de 10 bandas, modo Atmos compatível, saída combinada e aplicação direta na sessão via PipeWire."),
        ),
        (
            tr_mark("Limite atual"),
            tr("Diagnóstico de hardware, instalação do stack e correções de speaker ficam no Galaxy Book Setup. Aqui o foco é só o ajuste de som, sem Dolby Atmos proprietário."),
        ),
    ] {
        let row = adw::ActionRow::builder()
            .title(tr(title))
            .subtitle(subtitle)
            .build();
        row.set_activatable(false);
        row.set_subtitle_selectable(true);
        scope_group.add(&row);
    }

    page.add(&app_group);
    page.add(&scope_group);

    build_scrolled_navigation_page(&page, &tr("Detalhes"), "details")
}

pub(crate) fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        "
        .scale-value {
            min-width: 72px;
            font-feature-settings: 'tnum' 1;
        }
        ",
    );

    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
