use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, glib};

use crate::settings::{SHORTCUTS, Settings, Shortcut, Theme};
use crate::{apply_accels, apply_theme};

pub fn show(window: &adw::ApplicationWindow, settings: &Rc<RefCell<Settings>>) {
    let Some(app) = window.application().and_downcast::<adw::Application>() else {
        return;
    };
    let dialog = adw::PreferencesDialog::new();
    dialog.add(&general_page(&dialog, window, settings));
    dialog.add(&shortcuts_page(&dialog, &app, settings));
    dialog.present(Some(window));
}

fn general_page(dialog: &adw::PreferencesDialog, window: &adw::ApplicationWindow, settings: &Rc<RefCell<Settings>>) -> adw::PreferencesPage {
    let theme = adw::ComboRow::builder()
        .title("Theme")
        .model(&gtk::StringList::new(&["System", "Light", "Dark"]))
        .selected(Theme::ALL.iter().position(|t| *t == settings.borrow().theme).unwrap_or(0) as u32)
        .build();
    theme.connect_selected_notify(glib::clone!(
        #[strong] settings,
        #[weak] dialog,
        move |row| {
            let theme = Theme::ALL[row.selected() as usize];
            settings.borrow_mut().theme = theme;
            apply_theme(theme);
            save(&dialog, &settings.borrow());
        }
    ));
    let appearance = adw::PreferencesGroup::builder().title("Appearance").build();
    appearance.add(&theme);

    let export = adw::PreferencesGroup::builder()
        .title("Export")
        .description("Exports the rendered preview of the current document.")
        .build();
    for (title, action) in [("Export as PDF…", "export-pdf"), ("Export as PNG…", "export-png")] {
        let row = adw::ButtonRow::builder().title(title).build();
        row.connect_activated(glib::clone!(
            #[weak] dialog,
            #[weak] window,
            move |_| {
                dialog.close();
                WidgetExt::activate_action(&window, &format!("win.{action}"), None).ok();
            }
        ));
        export.add(&row);
    }

    let page = adw::PreferencesPage::builder().title("General").icon_name("preferences-system-symbolic").build();
    page.add(&appearance);
    page.add(&export);
    page
}

fn shortcuts_page(dialog: &adw::PreferencesDialog, app: &adw::Application, settings: &Rc<RefCell<Settings>>) -> adw::PreferencesPage {
    let group = adw::PreferencesGroup::builder()
        .title("Keyboard Shortcuts")
        .description("Click a shortcut to change it.")
        .build();
    let labels: Rc<Vec<gtk::Label>> = Rc::new(
        SHORTCUTS
            .iter()
            .map(|shortcut| {
                let label = gtk::Label::new(Some(&accel_label(settings.borrow().accel(shortcut.id))));
                label.add_css_class("dim-label");
                let row = adw::ActionRow::builder().title(shortcut.title).activatable(true).build();
                row.add_suffix(&label);
                row.connect_activated(glib::clone!(
                    #[weak] dialog,
                    #[weak] app,
                    #[weak] label,
                    #[strong] settings,
                    move |_| record(&dialog, &app, &settings, shortcut, &label)
                ));
                group.add(&row);
                label
            })
            .collect(),
    );

    let reset = adw::ButtonRow::builder().title("Reset to Defaults").build();
    reset.connect_activated(glib::clone!(
        #[weak] dialog,
        #[weak] app,
        #[strong] settings,
        move |_| {
            settings.borrow_mut().reset_accels();
            apply_accels(&app, &settings.borrow());
            for (label, shortcut) in labels.iter().zip(SHORTCUTS) {
                label.set_label(&accel_label(settings.borrow().accel(shortcut.id)));
            }
            save(&dialog, &settings.borrow());
        }
    ));
    let reset_group = adw::PreferencesGroup::new();
    reset_group.add(&reset);

    let page = adw::PreferencesPage::builder()
        .title("Shortcuts")
        .icon_name("preferences-desktop-keyboard-shortcuts-symbolic")
        .build();
    page.add(&group);
    page.add(&reset_group);
    page
}

fn record(dialog: &adw::PreferencesDialog, app: &adw::Application, settings: &Rc<RefCell<Settings>>, shortcut: &'static Shortcut, label: &gtk::Label) {
    for s in SHORTCUTS {
        app.set_accels_for_action(s.action, &[]);
    }
    let hint = "Press the new key combination.\nEsc cancels, Backspace removes the shortcut.";
    let prompt = adw::AlertDialog::new(Some(&format!("Shortcut for “{}”", shortcut.title)), Some(hint));
    prompt.add_response("cancel", "_Cancel");
    prompt.set_close_response("cancel");

    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    keys.connect_key_pressed(glib::clone!(
        #[weak] prompt,
        #[weak] dialog,
        #[weak] label,
        #[strong] settings,
        #[upgrade_or] glib::Propagation::Proceed,
        move |_, key, _, state| {
            let mods = state & gtk::accelerator_get_default_mod_mask();
            let accel = if mods.is_empty() && key == gdk::Key::Escape {
                prompt.close();
                return glib::Propagation::Stop;
            } else if mods.is_empty() && key == gdk::Key::BackSpace {
                String::new()
            } else if gtk::accelerator_valid(key, mods) && (!mods.is_empty() || is_function_key(key)) {
                gtk::accelerator_name(key.to_lower(), mods).to_string()
            } else {
                return glib::Propagation::Proceed;
            };
            if let Some(other) = settings.borrow().conflict(shortcut.id, &accel) {
                prompt.set_body(&format!("{} is already used by “{}”.\n{hint}", accel_label(&accel), other.title));
                return glib::Propagation::Stop;
            }
            settings.borrow_mut().set_accel(shortcut.id, &accel);
            label.set_label(&accel_label(&accel));
            save(&dialog, &settings.borrow());
            prompt.close();
            glib::Propagation::Stop
        }
    ));
    prompt.add_controller(keys);
    prompt.connect_closed(glib::clone!(
        #[weak] app,
        #[strong] settings,
        move |_| apply_accels(&app, &settings.borrow())
    ));
    prompt.present(Some(dialog));
}

fn is_function_key(key: gdk::Key) -> bool {
    key.name().is_some_and(|n| n.len() > 1 && n.starts_with('F') && n[1..].chars().all(|c| c.is_ascii_digit()))
}

fn accel_label(accel: &str) -> String {
    match gtk::accelerator_parse(accel) {
        Some((key, mods)) if !accel.is_empty() => gtk::accelerator_get_label(key, mods).to_string(),
        _ => "Disabled".to_string(),
    }
}

fn save(dialog: &adw::PreferencesDialog, settings: &Settings) {
    if let Err(err) = settings.save() {
        dialog.add_toast(adw::Toast::new(&format!("Could not save settings: {err}")));
    }
}
