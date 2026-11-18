mod render;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use gtk::glib;
use sourceview5::prelude::*;
use webkit6::prelude::*;

const APP_ID: &str = "io.github.GreisMicha.downmark";
const RENDER_DELAY: Duration = Duration::from_millis(50);

fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_window);
    app.set_accels_for_action("win.mode::editor", &["<Ctrl>1"]);
    app.set_accels_for_action("win.mode::split", &["<Ctrl>2"]);
    app.set_accels_for_action("win.mode::preview", &["<Ctrl>3"]);
    app.run()
}

fn build_window(app: &adw::Application) {
    let buffer = sourceview5::Buffer::new(None);
    buffer.set_language(sourceview5::LanguageManager::default().language("markdown").as_ref());
    follow_dark_mode(&buffer);

    let editor = sourceview5::View::with_buffer(&buffer);
    editor.set_monospace(true);
    editor.set_show_line_numbers(true);
    editor.set_highlight_current_line(true);
    editor.set_wrap_mode(gtk::WrapMode::WordChar);
    editor.set_top_margin(8);
    editor.set_left_margin(8);
    let editor_scroll = gtk::ScrolledWindow::builder().child(&editor).build();

    let preview = webkit6::WebView::new();
    if let Some(settings) = WebViewExt::settings(&preview) {
        settings.set_enable_javascript_markup(false);
    }
    preview.load_html(&render::page(""), None);

    let paned = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(&editor_scroll)
        .end_child(&preview)
        .shrink_start_child(false)
        .shrink_end_child(false)
        .position(600)
        .build();

    let modes = adw::ToggleGroup::new();
    for (name, icon, tooltip) in [
        ("editor", "document-edit-symbolic", "Editor (Ctrl+1)"),
        ("split", "view-dual-symbolic", "Split (Ctrl+2)"),
        ("preview", "view-reveal-symbolic", "Preview (Ctrl+3)"),
    ] {
        modes.add(adw::Toggle::builder().name(name).icon_name(icon).tooltip(tooltip).build());
    }
    modes.connect_active_name_notify(glib::clone!(
        #[weak] editor_scroll,
        #[weak] preview,
        move |modes| {
            let mode = modes.active_name().unwrap_or_default();
            editor_scroll.set_visible(mode != "preview");
            preview.set_visible(mode != "editor");
        }
    ));
    modes.set_active_name(Some("split"));

    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&modes));
    let content = adw::ToolbarView::new();
    content.add_top_bar(&header);
    content.set_content(Some(&paned));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("downmark")
        .default_width(1200)
        .default_height(800)
        .content(&content)
        .build();

    let mode_action = gtk::gio::SimpleAction::new("mode", Some(glib::VariantTy::STRING));
    mode_action.connect_activate(glib::clone!(
        #[weak] modes,
        move |_, param| modes.set_active_name(param.and_then(|p| p.str()))
    ));
    window.add_action(&mode_action);

    let pending: Rc<RefCell<Option<glib::SourceId>>> = Rc::default();
    buffer.connect_changed(glib::clone!(
        #[weak] preview,
        move |buffer| {
            if let Some(id) = pending.take() {
                id.remove();
            }
            let buffer = buffer.clone();
            let preview = preview.clone();
            let pending_inner = pending.clone();
            pending.replace(Some(glib::timeout_add_local_once(RENDER_DELAY, move || {
                pending_inner.take();
                let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
                let script = render::update_script(&render::to_html(&text));
                preview.evaluate_javascript(&script, None, None, None::<&gtk::gio::Cancellable>, |_| {});
            })));
        }
    ));

    window.present();
}

fn follow_dark_mode(buffer: &sourceview5::Buffer) {
    let style = adw::StyleManager::default();
    let apply = glib::clone!(
        #[weak] buffer,
        move |style: &adw::StyleManager| {
            let name = if style.is_dark() { "Adwaita-dark" } else { "Adwaita" };
            buffer.set_style_scheme(sourceview5::StyleSchemeManager::default().scheme(name).as_ref());
        }
    );
    apply(&style);
    style.connect_dark_notify(apply);
}
