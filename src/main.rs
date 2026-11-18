mod highlight;
mod prefs;
mod render;
mod settings;
mod window;

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use adw::prelude::*;
use gtk::{gio, glib};

use settings::{SHORTCUTS, Settings, Theme};

const APP_ID: &str = "io.github.SirOpolov.downmark";

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
    let current: Rc<RefCell<Weak<window::Window>>> = Rc::default();
    let window = move |app: &adw::Application| {
        if let Some(win) = current.borrow().upgrade() {
            return win;
        }
        let win = window::Window::new(app, &settings);
        current.replace(Rc::downgrade(&win));
        win
    };
    let window = Rc::new(window);
    app.connect_activate(glib::clone!(
        #[strong] window,
        move |app| window(app).present()
    ));
    app.connect_open(move |app, files, _| {
        let win = window(app);
        for file in files {
            win.open(file);
        }
        win.present();
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
