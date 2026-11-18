mod render;
mod window;

use adw::prelude::*;
use gtk::{gio, glib};

const APP_ID: &str = "io.github.GreisMicha.downmark";

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();
    app.connect_activate(|app| window::Doc::new(app).present());
    app.connect_open(|app, files, _| {
        for file in files {
            let doc = window::Doc::new(app);
            doc.open(file);
            doc.present();
        }
    });
    app.set_accels_for_action("win.open", &["<Control>o"]);
    app.set_accels_for_action("win.mode::editor", &["<Control>1"]);
    app.set_accels_for_action("win.mode::split", &["<Control>2"]);
    app.set_accels_for_action("win.mode::preview", &["<Control>3"]);
    app.run()
}
