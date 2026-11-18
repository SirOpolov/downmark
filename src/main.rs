mod highlight;
mod prefs;
mod render;
mod settings;
mod window;

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};

use settings::{SHORTCUTS, Settings, Theme};

const APP_ID: &str = "io.github.GreisMicha.downmark";

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();
    let settings = Rc::new(RefCell::new(Settings::load()));

    app.connect_startup(glib::clone!(
        #[strong] settings,
        move |app| {
            if let Err(err) = highlight::install() {
                eprintln!("downmark: syntax highlighting falls back to stock Markdown: {err}");
            }
            apply_accels(app, &settings.borrow());
            apply_theme(settings.borrow().theme);
        }
    ));
    app.connect_activate(glib::clone!(
        #[strong] settings,
        move |app| window::Doc::new(app, &settings).present()
    ));
    app.connect_open(move |app, files, _| {
        for file in files {
            let doc = window::Doc::new(app, &settings);
            doc.open(file);
            doc.present();
        }
    });
    app.run()
}

pub fn apply_accels(app: &adw::Application, settings: &Settings) {
    for shortcut in SHORTCUTS {
        let accel = settings.accel(shortcut.id);
        let accels: &[&str] = if accel.is_empty() { &[] } else { &[accel] };
        app.set_accels_for_action(shortcut.action, accels);
    }
}

pub fn apply_theme(theme: Theme) {
    adw::StyleManager::default().set_color_scheme(match theme {
        Theme::System => adw::ColorScheme::Default,
        Theme::Light => adw::ColorScheme::ForceLight,
        Theme::Dark => adw::ColorScheme::ForceDark,
    });
}
