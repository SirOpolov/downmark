use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use gtk::{gio, glib};
use sourceview5::prelude::*;
use webkit6::prelude::*;

use crate::render;
use crate::settings::Settings;

const RENDER_DELAY: Duration = Duration::from_millis(50);

pub struct Doc {
    window: adw::ApplicationWindow,
    title: adw::WindowTitle,
    buffer: sourceview5::Buffer,
    preview: webkit6::WebView,
    modes: adw::ToggleGroup,
    toasts: adw::ToastOverlay,
    settings: Rc<RefCell<Settings>>,
    file: RefCell<Option<gio::File>>,
    pending: RefCell<Option<glib::SourceId>>,
}

impl Doc {
    pub fn new(app: &adw::Application, settings: &Rc<RefCell<Settings>>) -> Rc<Self> {
        let buffer = sourceview5::Buffer::new(None);
        buffer.set_language(sourceview5::LanguageManager::default().language("markdown").as_ref());

        let editor = sourceview5::View::with_buffer(&buffer);
        editor.set_monospace(true);
        editor.set_show_line_numbers(true);
        editor.set_highlight_current_line(true);
        editor.set_wrap_mode(gtk::WrapMode::WordChar);
        editor.set_top_margin(render::TOP_MARGIN);
        editor.set_left_margin(8);
        let editor_scroll = gtk::ScrolledWindow::builder().child(&editor).build();

        let preview = webkit6::WebView::new();
        if let Some(settings) = WebViewExt::settings(&preview) {
            settings.set_enable_javascript_markup(false);
        }

        let paned = gtk::Paned::builder()
            .orientation(gtk::Orientation::Horizontal)
            .start_child(&editor_scroll)
            .end_child(&preview)
            .shrink_start_child(false)
            .shrink_end_child(false)
            .position(600)
            .build();

        let modes = adw::ToggleGroup::new();
        for (name, icon) in [
            ("editor", "document-edit-symbolic"),
            ("split", "view-dual-symbolic"),
            ("preview", "view-reveal-symbolic"),
        ] {
            modes.add(adw::Toggle::builder().name(name).icon_name(icon).build());
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

        let title = adw::WindowTitle::new("", "");
        let header = adw::HeaderBar::new();
        header.set_title_widget(Some(&title));
        header.pack_end(&gtk::MenuButton::builder().icon_name("open-menu-symbolic").menu_model(&main_menu()).primary(true).build());
        header.pack_end(&modes);

        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&paned));
        let content = adw::ToolbarView::new();
        content.add_top_bar(&header);
        content.set_content(Some(&toasts));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .default_width(1200)
            .default_height(800)
            .content(&content)
            .build();

        let doc = Rc::new(Doc {
            window,
            title,
            buffer,
            preview,
            modes,
            toasts,
            settings: settings.clone(),
            file: RefCell::default(),
            pending: RefCell::default(),
        });
        doc.connect_signals();
        doc.follow_dark_mode();
        doc.update_title();
        doc.reload_preview();
        doc
    }

    pub fn present(&self) {
        self.window.present();
    }

    fn action(self: &Rc<Self>, name: &str, param: Option<&glib::VariantTy>, f: impl Fn(&Rc<Self>, Option<&glib::Variant>) + 'static) {
        let action = gio::SimpleAction::new(name, param);
        let weak = Rc::downgrade(self);
        action.connect_activate(move |_, param| {
            if let Some(doc) = weak.upgrade() {
                f(&doc, param);
            }
        });
        self.window.add_action(&action);
    }

    fn connect_signals(self: &Rc<Self>) {
        self.action("mode", Some(glib::VariantTy::STRING), |doc, param| {
            doc.modes.set_active_name(param.and_then(|p| p.str()));
        });
        self.action("open", None, |doc, _| doc.choose_file_to_open());
        self.action("export-pdf", None, |doc, _| doc.choose_export("pdf"));
        self.action("export-png", None, |doc, _| doc.choose_export("png"));
        self.action("settings", None, |doc, _| crate::prefs::show(&doc.window, &doc.settings));
        self.action("save", None, |doc, _| doc.save(|_| {}));
        self.action("save-as", None, |doc, _| doc.save_as(|_| {}));
        self.action("new", None, |doc, _| {
            if let Some(app) = doc.window.application().and_downcast::<adw::Application>() {
                Doc::new(&app, &doc.settings).present();
            }
        });

        let weak = Rc::downgrade(self);
        self.buffer.connect_changed(move |_| {
            if let Some(doc) = weak.upgrade() {
                doc.schedule_render();
            }
        });
        let weak = Rc::downgrade(self);
        self.buffer.connect_modified_changed(move |_| {
            if let Some(doc) = weak.upgrade() {
                doc.update_title();
            }
        });

        let weak = Rc::downgrade(self);
        self.preview.connect_decide_policy(move |_, decision, kind| {
            kind == webkit6::PolicyDecisionType::NavigationAction
                && weak.upgrade().is_some_and(|doc| doc.handle_link(decision))
        });

        let keep_alive = RefCell::new(Some(self.clone()));
        self.window.connect_close_request(move |_| {
            let Some(doc) = keep_alive.borrow().clone() else {
                return glib::Propagation::Proceed;
            };
            if doc.buffer.is_modified() {
                doc.confirm_close();
                return glib::Propagation::Stop;
            }
            keep_alive.take();
            glib::Propagation::Proceed
        });
    }

    fn follow_dark_mode(self: &Rc<Self>) {
        let style = adw::StyleManager::default();
        self.apply_dark(style.is_dark());
        let weak = Rc::downgrade(self);
        style.connect_dark_notify(move |style| {
            if let Some(doc) = weak.upgrade() {
                doc.apply_dark(style.is_dark());
            }
        });
    }

    fn apply_dark(&self, dark: bool) {
        let name = if dark { "Adwaita-dark" } else { "Adwaita" };
        self.buffer.set_style_scheme(sourceview5::StyleSchemeManager::default().scheme(name).as_ref());
    }

    fn text(&self) -> String {
        self.buffer.text(&self.buffer.start_iter(), &self.buffer.end_iter(), false).into()
    }

    fn file_name(&self) -> Option<String> {
        self.file.borrow().as_ref().and_then(|f| f.basename()).map(|p| p.to_string_lossy().into_owned())
    }

    fn update_title(&self) {
        let title = title(self.file_name().as_deref(), self.buffer.is_modified());
        let dir = self.file.borrow().as_ref().and_then(|f| f.parent()).and_then(|p| p.path());
        self.title.set_title(&title);
        self.title.set_subtitle(&dir.map(|d| d.display().to_string()).unwrap_or_default());
        self.window.set_title(Some(&title));
    }

    fn toast(&self, message: &str) {
        self.toasts.add_toast(adw::Toast::new(message));
    }

    fn schedule_render(self: &Rc<Self>) {
        if let Some(id) = self.pending.take() {
            id.remove();
        }
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(RENDER_DELAY, move || {
            if let Some(doc) = weak.upgrade() {
                doc.pending.take();
                let script = render::update_script(&render::to_html(&doc.text()));
                doc.preview.evaluate_javascript(&script, None, None, None::<&gio::Cancellable>, |_| {});
            }
        });
        self.pending.replace(Some(id));
    }

    fn reload_preview(&self) {
        let base = self.file.borrow().as_ref().and_then(base_uri);
        self.preview.load_html(&render::page(&render::to_html(&self.text())), base.as_deref());
    }

    fn handle_link(&self, decision: &webkit6::PolicyDecision) -> bool {
        let Some(action) = decision
            .downcast_ref::<webkit6::NavigationPolicyDecision>()
            .and_then(|d| d.navigation_action())
        else {
            return false;
        };
        if action.navigation_type() != webkit6::NavigationType::LinkClicked {
            return false;
        }
        let Some(uri) = action.request().and_then(|r| r.uri()) else {
            return false;
        };
        if is_internal(&uri, &self.preview.uri().unwrap_or_default()) {
            return false;
        }
        decision.ignore();
        if is_launchable(&uri) {
            gtk::UriLauncher::new(&uri).launch(Some(&self.window), None::<&gio::Cancellable>, |_| {});
        }
        true
    }

    fn is_pristine(&self) -> bool {
        self.file.borrow().is_none() && !self.buffer.is_modified() && self.buffer.char_count() == 0
    }

    fn choose_file_to_open(self: &Rc<Self>) {
        let dialog = gtk::FileDialog::builder().title("Open Markdown File").default_filter(&markdown_filter()).build();
        let weak = Rc::downgrade(self);
        dialog.open(Some(&self.window), None::<&gio::Cancellable>, move |result| {
            if let (Some(doc), Ok(file)) = (weak.upgrade(), result) {
                doc.open_here_or_new_window(file);
            }
        });
    }

    fn save(self: &Rc<Self>, after: fn(&Rc<Self>)) {
        let file = self.file.borrow().clone();
        match file {
            Some(file) => {
                if self.write_to(&file) {
                    after(self);
                }
            }
            None => self.save_as(after),
        }
    }

    fn save_as(self: &Rc<Self>, after: fn(&Rc<Self>)) {
        let dialog = gtk::FileDialog::builder()
            .title("Save As")
            .initial_name(self.file_name().unwrap_or_else(|| "Untitled.md".into()))
            .default_filter(&markdown_filter())
            .build();
        if let Some(dir) = self.file.borrow().as_ref().and_then(|f| f.parent()) {
            dialog.set_initial_folder(Some(&dir));
        }
        let weak = Rc::downgrade(self);
        dialog.save(Some(&self.window), None::<&gio::Cancellable>, move |result| {
            if let (Some(doc), Ok(file)) = (weak.upgrade(), result)
                && doc.write_to(&file)
            {
                after(&doc);
            }
        });
    }

    fn write_to(&self, file: &gio::File) -> bool {
        if let Err(err) = write_text(file, &self.text()) {
            self.toast(&format!("Could not save: {}", err.message()));
            return false;
        }
        let old_dir = self.file.replace(Some(file.clone())).and_then(|f| f.parent());
        self.buffer.set_modified(false);
        self.update_title();
        if old_dir.as_ref().map(|d| d.uri()) != file.parent().map(|d| d.uri()) {
            self.reload_preview();
        }
        true
    }

    fn choose_export(self: &Rc<Self>, ext: &'static str) {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some(&ext.to_uppercase()));
        filter.add_suffix(ext);
        let dialog = gtk::FileDialog::builder()
            .title(format!("Export as {}", ext.to_uppercase()))
            .initial_name(export_name(self.file_name().as_deref(), ext))
            .default_filter(&filter)
            .build();
        if let Some(dir) = self.file.borrow().as_ref().and_then(|f| f.parent()) {
            dialog.set_initial_folder(Some(&dir));
        }
        let weak = Rc::downgrade(self);
        dialog.save(Some(&self.window), None::<&gio::Cancellable>, move |result| {
            if let (Some(doc), Ok(file)) = (weak.upgrade(), result) {
                if ext == "pdf" {
                    doc.export_pdf(&file);
                } else {
                    doc.export_png(&file);
                }
            }
        });
    }

    fn export_png(self: &Rc<Self>, file: &gio::File) {
        let Some(path) = file.path() else {
            self.toast("Export failed: not a local file");
            return;
        };
        let weak = Rc::downgrade(self);
        self.preview.snapshot(
            webkit6::SnapshotRegion::FullDocument,
            webkit6::SnapshotOptions::NONE,
            None::<&gio::Cancellable>,
            move |result| {
                let Some(doc) = weak.upgrade() else { return };
                let saved = result
                    .map_err(|e| e.message().to_string())
                    .and_then(|texture| texture.save_to_png(&path).map_err(|e| e.message.to_string()));
                match saved {
                    Ok(()) => doc.toast(&format!("Exported {}", path.display())),
                    Err(err) => doc.toast(&format!("Export failed: {err}")),
                }
            },
        );
    }

    fn export_pdf(self: &Rc<Self>, file: &gio::File) {
        let settings = gtk::PrintSettings::new();
        settings.set_printer(&glib::dgettext(Some("gtk40"), "Print to File"));
        settings.set("output-file-format", Some("pdf"));
        settings.set("output-uri", Some(&file.uri()));
        let operation = webkit6::PrintOperation::new(&self.preview);
        operation.set_print_settings(&settings);

        let failed = Rc::new(std::cell::Cell::new(false));
        let weak = Rc::downgrade(self);
        operation.connect_failed(glib::clone!(
            #[strong] failed,
            move |_, err| {
                failed.set(true);
                if let Some(doc) = weak.upgrade() {
                    doc.toast(&format!("Export failed: {}", err.message()));
                }
            }
        ));
        let weak = Rc::downgrade(self);
        let target = file.path().map(|p| p.display().to_string()).unwrap_or_else(|| file.uri().into());
        let keep_alive = RefCell::new(Some(operation.clone()));
        operation.connect_finished(move |_| {
            keep_alive.take();
            if let (Some(doc), false) = (weak.upgrade(), failed.get()) {
                doc.toast(&format!("Exported {target}"));
            }
        });
        operation.print();
    }

    fn confirm_close(self: &Rc<Self>) {
        let name = self.file_name().unwrap_or_else(|| "Untitled".into());
        let dialog = adw::AlertDialog::new(Some("Save changes?"), Some(&format!("{name} has unsaved changes.")));
        dialog.add_responses(&[("cancel", "_Cancel"), ("discard", "_Discard"), ("save", "_Save")]);
        dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
        dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("save"));
        dialog.set_close_response("cancel");
        let weak = Rc::downgrade(self);
        dialog.choose(Some(&self.window), None::<&gio::Cancellable>, move |response| {
            let Some(doc) = weak.upgrade() else { return };
            match response.as_str() {
                "discard" => {
                    doc.buffer.set_modified(false);
                    doc.window.close();
                }
                "save" => doc.save(|doc| doc.window.close()),
                _ => {}
            }
        });
    }

    pub fn open_here_or_new_window(self: &Rc<Self>, file: gio::File) {
        if self.is_pristine() {
            self.open(&file);
        } else if let Some(app) = self.window.application().and_downcast::<adw::Application>() {
            let doc = Doc::new(&app, &self.settings);
            doc.open(&file);
            doc.present();
        }
    }

    pub fn open(&self, file: &gio::File) {
        match read_text(file) {
            Ok(text) => {
                self.buffer.begin_irreversible_action();
                self.buffer.set_text(&text);
                self.buffer.end_irreversible_action();
                self.buffer.place_cursor(&self.buffer.start_iter());
                self.buffer.set_modified(false);
                self.file.replace(Some(file.clone()));
                self.update_title();
                self.reload_preview();
            }
            Err(err) => {
                let name = file.basename().map(|p| p.display().to_string()).unwrap_or_default();
                self.toast(&format!("Could not open {name}: {err}"));
            }
        }
    }
}

fn main_menu() -> gio::Menu {
    let files = gio::Menu::new();
    files.append(Some("New Window"), Some("win.new"));
    files.append(Some("Open…"), Some("win.open"));
    files.append(Some("Save"), Some("win.save"));
    files.append(Some("Save As…"), Some("win.save-as"));
    let export = gio::Menu::new();
    export.append(Some("Export as PDF…"), Some("win.export-pdf"));
    export.append(Some("Export as PNG…"), Some("win.export-png"));
    let app = gio::Menu::new();
    app.append(Some("Settings"), Some("win.settings"));
    let menu = gio::Menu::new();
    menu.append_section(None, &files);
    menu.append_section(None, &export);
    menu.append_section(None, &app);
    menu
}

fn markdown_filter() -> gtk::FileFilter {
    let filter = gtk::FileFilter::new();
    filter.set_name(Some("Markdown"));
    filter.add_mime_type("text/markdown");
    filter.add_suffix("md");
    filter.add_suffix("markdown");
    filter
}

pub fn export_name(name: Option<&str>, ext: &str) -> String {
    let stem = name
        .and_then(|n| std::path::Path::new(n).file_stem())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Untitled".into());
    format!("{stem}.{ext}")
}

pub fn title(name: Option<&str>, modified: bool) -> String {
    let name = name.unwrap_or("Untitled");
    if modified { format!("• {name}") } else { name.to_string() }
}

pub fn base_uri(file: &gio::File) -> Option<String> {
    let dir = file.parent()?.uri();
    Some(if dir.ends_with('/') { dir.into() } else { format!("{dir}/") })
}

pub fn is_internal(uri: &str, page: &str) -> bool {
    uri.split_once('#').is_some_and(|(path, _)| path == page)
}

pub fn is_launchable(uri: &str) -> bool {
    let scheme = uri.split_once(':').map(|(s, _)| s.to_ascii_lowercase()).unwrap_or_default();
    matches!(scheme.as_str(), "http" | "https" | "mailto" | "file")
}

pub fn write_text(file: &gio::File, text: &str) -> Result<(), glib::Error> {
    file.replace_contents(text.as_bytes(), None, false, gio::FileCreateFlags::NONE, None::<&gio::Cancellable>)
        .map(|_| ())
}

pub fn read_text(file: &gio::File) -> Result<String, String> {
    let (bytes, _) = file.load_contents(None::<&gio::Cancellable>).map_err(|e| e.message().to_string())?;
    String::from_utf8(bytes.to_vec()).map_err(|_| "not a UTF-8 text file".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str, bytes: &[u8]) -> gio::File {
        let path = std::env::temp_dir().join(format!("downmark-test-{}-{name}", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        gio::File::for_path(path)
    }

    #[test]
    fn export_name_replaces_extension() {
        assert_eq!(export_name(Some("README.md"), "pdf"), "README.pdf");
        assert_eq!(export_name(Some("notes.v2.markdown"), "png"), "notes.v2.png");
    }

    #[test]
    fn export_name_without_file() {
        assert_eq!(export_name(None, "pdf"), "Untitled.pdf");
        assert_eq!(export_name(Some("NOTES"), "png"), "NOTES.png");
    }

    #[test]
    fn title_shows_file_name() {
        assert_eq!(title(Some("README.md"), false), "README.md");
    }

    #[test]
    fn title_marks_modified() {
        assert_eq!(title(Some("README.md"), true), "• README.md");
    }

    #[test]
    fn title_untitled() {
        assert_eq!(title(None, false), "Untitled");
    }

    #[test]
    fn read_text_returns_utf8_content() {
        let file = temp_file("ok.md", "# Grüße\n".as_bytes());
        assert_eq!(read_text(&file).unwrap(), "# Grüße\n");
    }

    #[test]
    fn read_text_rejects_binary() {
        let file = temp_file("bin.md", &[0xff, 0xfe, 0x00, 0x80]);
        assert_eq!(read_text(&file).unwrap_err(), "not a UTF-8 text file");
    }

    #[test]
    fn base_uri_is_parent_directory_with_trailing_slash() {
        let file = gio::File::for_path("/home/user/notes/a b.md");
        assert_eq!(base_uri(&file).unwrap(), "file:///home/user/notes/");
    }

    #[test]
    fn base_uri_of_file_in_root() {
        assert_eq!(base_uri(&gio::File::for_path("/a.md")).unwrap(), "file:///");
    }

    #[test]
    fn fragment_link_on_same_page_is_internal() {
        assert!(is_internal("file:///notes/#fn1", "file:///notes/"));
        assert!(is_internal("about:blank#fn1", "about:blank"));
    }

    #[test]
    fn only_web_mail_and_file_links_are_launchable() {
        assert!(is_launchable("https://example.com"));
        assert!(is_launchable("http://example.com"));
        assert!(is_launchable("mailto:a@b.c"));
        assert!(is_launchable("file:///notes/other.md"));
        assert!(!is_launchable("javascript:alert(1)"));
        assert!(!is_launchable("JavaScript:alert(1)"));
        assert!(!is_launchable("data:text/html,x"));
    }

    #[test]
    fn other_links_are_external() {
        assert!(!is_internal("https://example.com/#x", "file:///notes/"));
        assert!(!is_internal("file:///notes/other.md", "file:///notes/"));
        assert!(!is_internal("file:///notes/", "file:///notes/"));
    }

    #[test]
    fn write_text_overwrites_file_with_exact_content() {
        let file = temp_file("save.md", b"old content that is longer");
        write_text(&file, "new ✓\n").unwrap();
        assert_eq!(std::fs::read(file.path().unwrap()).unwrap(), "new ✓\n".as_bytes());
    }

    #[test]
    fn write_text_fails_in_missing_directory() {
        let file = gio::File::for_path("/nonexistent/dir/downmark.md");
        assert!(write_text(&file, "x").is_err());
    }

    #[test]
    fn read_text_reports_missing_file() {
        let file = gio::File::for_path("/nonexistent/downmark.md");
        assert!(!read_text(&file).unwrap_err().is_empty());
    }
}
