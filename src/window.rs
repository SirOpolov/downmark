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

pub struct Window {
    window: adw::ApplicationWindow,
    title: adw::WindowTitle,
    tabs: adw::TabView,
    modes: adw::ToggleGroup,
    toasts: adw::ToastOverlay,
    settings: Rc<RefCell<Settings>>,
    docs: RefCell<Vec<Rc<Doc>>>,
}

impl Window {
    pub fn new(app: &adw::Application, settings: &Rc<RefCell<Settings>>) -> Rc<Self> {
        let modes = adw::ToggleGroup::new();
        for (name, icon) in [
            ("editor", "document-edit-symbolic"),
            ("split", "view-dual-symbolic"),
            ("preview", "view-reveal-symbolic"),
        ] {
            modes.add(adw::Toggle::builder().name(name).icon_name(icon).build());
        }
        modes.set_active_name(Some(&settings.borrow().mode));

        let title = adw::WindowTitle::new("", "");
        let header = adw::HeaderBar::new();
        header.set_title_widget(Some(&title));
        header.pack_end(&gtk::MenuButton::builder().icon_name("open-menu-symbolic").menu_model(&main_menu()).primary(true).build());
        header.pack_end(&modes);

        let tabs = adw::TabView::new();
        let tab_bar = adw::TabBar::builder().view(&tabs).autohide(false).build();
        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&tabs));
        let content = adw::ToolbarView::new();
        content.add_top_bar(&header);
        content.add_top_bar(&tab_bar);
        content.set_content(Some(&toasts));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .default_width(settings.borrow().width)
            .default_height(settings.borrow().height)
            .maximized(settings.borrow().maximized)
            .content(&content)
            .build();

        let win = Rc::new(Window {
            window,
            title,
            tabs,
            modes,
            toasts,
            settings: settings.clone(),
            docs: RefCell::default(),
        });
        win.connect_signals();
        win.add_doc();
        win
    }

    pub fn present(&self) {
        self.window.present();
    }

    fn action(self: &Rc<Self>, name: &str, param: Option<&glib::VariantTy>, f: impl Fn(&Rc<Self>, Option<&glib::Variant>) + 'static) {
        let action = gio::SimpleAction::new(name, param);
        let weak = Rc::downgrade(self);
        action.connect_activate(move |_, param| {
            if let Some(win) = weak.upgrade() {
                f(&win, param);
            }
        });
        self.window.add_action(&action);
    }

    fn doc_action(self: &Rc<Self>, name: &str, f: impl Fn(&Rc<Doc>) + 'static) {
        self.action(name, None, move |win, _| {
            if let Some(doc) = win.current() {
                f(&doc);
            }
        });
    }

    fn connect_signals(self: &Rc<Self>) {
        self.action("mode", Some(glib::VariantTy::STRING), |win, param| {
            win.modes.set_active_name(param.and_then(|p| p.str()));
        });
        self.action("new", None, |win, _| {
            win.add_doc();
        });
        self.action("open", None, |win, _| win.choose_files_to_open());
        self.action("close-tab", None, |win, _| {
            if let Some(page) = win.tabs.selected_page() {
                win.tabs.close_page(&page);
            }
        });
        self.action("settings", None, |win, _| crate::prefs::show(&win.window, &win.settings));
        self.doc_action("save", |doc| doc.save(|_| {}));
        self.doc_action("save-as", |doc| doc.save_as(|_| {}));
        self.doc_action("export-pdf", |doc| doc.choose_export("pdf"));
        self.doc_action("export-png", |doc| doc.choose_export("png"));

        let weak = Rc::downgrade(self);
        self.modes.connect_active_name_notify(move |modes| {
            if let Some(win) = weak.upgrade() {
                let mode = modes.active_name().unwrap_or_default();
                for doc in win.docs.borrow().iter() {
                    doc.set_mode(&mode);
                }
            }
        });

        let weak = Rc::downgrade(self);
        adw::StyleManager::default().connect_dark_notify(move |style| {
            if let Some(win) = weak.upgrade() {
                for doc in win.docs.borrow().iter() {
                    doc.apply_dark(style.is_dark());
                }
            }
        });

        let weak = Rc::downgrade(self);
        self.tabs.connect_selected_page_notify(move |_| {
            if let Some(doc) = weak.upgrade().and_then(|win| win.current()) {
                doc.update_title();
            }
        });

        let weak = Rc::downgrade(self);
        self.tabs.connect_close_page(move |tabs, page| {
            match weak.upgrade().and_then(|win| win.doc_for(page).map(|doc| (win, doc))) {
                Some((win, doc)) if doc.buffer.is_modified() => {
                    tabs.close_page_finish(page, false);
                    win.confirm_close(&doc, |win, doc| win.tabs.close_page(&doc.page));
                }
                _ => tabs.close_page_finish(page, true),
            }
            glib::Propagation::Stop
        });

        let weak = Rc::downgrade(self);
        self.tabs.connect_page_detached(move |tabs, page, _| {
            if let Some(win) = weak.upgrade() {
                win.docs.borrow_mut().retain(|doc| &doc.page != page);
                if tabs.n_pages() == 0 {
                    win.window.close();
                }
            }
        });

        let keep_alive = RefCell::new(Some(self.clone()));
        self.window.connect_close_request(move |_| {
            let Some(win) = keep_alive.borrow().clone() else {
                return glib::Propagation::Proceed;
            };
            let modified = win.docs.borrow().iter().find(|doc| doc.buffer.is_modified()).cloned();
            if let Some(doc) = modified {
                win.confirm_close(&doc, |win, _| win.window.close());
                return glib::Propagation::Stop;
            }
            win.save_state();
            keep_alive.take();
            glib::Propagation::Proceed
        });
    }

    fn save_state(&self) {
        let (width, height) = self.window.default_size();
        let mut settings = self.settings.borrow_mut();
        settings.width = width;
        settings.height = height;
        settings.maximized = self.window.is_maximized();
        settings.mode = self.modes.active_name().unwrap_or_default().to_string();
        if let Err(err) = settings.save() {
            eprintln!("downmark: could not save window state: {err}");
        }
    }

    fn current(&self) -> Option<Rc<Doc>> {
        self.doc_for(&self.tabs.selected_page()?)
    }

    fn doc_for(&self, page: &adw::TabPage) -> Option<Rc<Doc>> {
        self.docs.borrow().iter().find(|doc| &doc.page == page).cloned()
    }

    fn add_doc(&self) -> Rc<Doc> {
        let doc = Doc::new(self);
        self.docs.borrow_mut().push(doc.clone());
        self.tabs.set_selected_page(&doc.page);
        doc.update_title();
        doc.focus();
        doc
    }

    pub fn open(&self, file: &gio::File) {
        let open = self.docs.borrow().iter().find(|doc| doc.file.borrow().as_ref().is_some_and(|f| f.equal(file))).cloned();
        if let Some(doc) = open {
            self.tabs.set_selected_page(&doc.page);
            return;
        }
        let doc = match self.current() {
            Some(doc) if doc.is_pristine() => doc,
            _ => self.add_doc(),
        };
        if !doc.open(file) && doc.is_pristine() && self.tabs.n_pages() > 1 {
            self.tabs.close_page(&doc.page);
        }
    }

    fn choose_files_to_open(self: &Rc<Self>) {
        let dialog = gtk::FileDialog::builder().title("Open Markdown Files").default_filter(&markdown_filter()).build();
        let weak = Rc::downgrade(self);
        dialog.open_multiple(Some(&self.window), None::<&gio::Cancellable>, move |result| {
            if let (Some(win), Ok(files)) = (weak.upgrade(), result) {
                for file in files.iter::<gio::File>().flatten() {
                    win.open(&file);
                }
            }
        });
    }

    fn confirm_close(self: &Rc<Self>, doc: &Rc<Doc>, after: fn(&Rc<Self>, &Rc<Doc>)) {
        self.tabs.set_selected_page(&doc.page);
        let name = doc.file_name().unwrap_or_else(|| "Untitled".into());
        let body = if doc.file.borrow().is_some() {
            format!("{name} has unsaved changes.")
        } else {
            format!("{name} was never saved. Save it to keep it, otherwise it is discarded.")
        };
        let dialog = adw::AlertDialog::new(Some("Save changes?"), Some(&body));
        dialog.add_responses(&[("cancel", "_Cancel"), ("discard", "_Discard"), ("save", "_Save")]);
        dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
        dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("save"));
        dialog.set_close_response("cancel");
        let weak = Rc::downgrade(self);
        let doc_weak = Rc::downgrade(doc);
        dialog.choose(Some(&self.window), None::<&gio::Cancellable>, move |response| {
            let (Some(win), Some(doc)) = (weak.upgrade(), doc_weak.upgrade()) else { return };
            match response.as_str() {
                "discard" => {
                    doc.buffer.set_modified(false);
                    after(&win, &doc);
                }
                "save" => doc.save(move |doc| {
                    if let Some(win) = weak.upgrade() {
                        after(&win, doc);
                    }
                }),
                _ => {}
            }
        });
    }
}

pub struct Doc {
    window: adw::ApplicationWindow,
    title: adw::WindowTitle,
    toasts: adw::ToastOverlay,
    page: adw::TabPage,
    editor: gtk::ScrolledWindow,
    view: sourceview5::View,
    buffer: sourceview5::Buffer,
    preview: webkit6::WebView,
    file: RefCell<Option<gio::File>>,
    pending: RefCell<Option<glib::SourceId>>,
}

impl Doc {
    fn new(win: &Window) -> Rc<Self> {
        let buffer = sourceview5::Buffer::new(None);
        let languages = sourceview5::LanguageManager::default();
        buffer.set_language(languages.language("downmark").or_else(|| languages.language("markdown")).as_ref());

        let view = sourceview5::View::with_buffer(&buffer);
        view.set_monospace(true);
        view.set_show_line_numbers(true);
        view.set_highlight_current_line(true);
        view.set_wrap_mode(gtk::WrapMode::WordChar);
        view.set_top_margin(render::TOP_MARGIN);
        view.set_left_margin(8);
        let editor = gtk::ScrolledWindow::builder().child(&view).build();

        let preview = webkit6::WebView::new();
        if let Some(settings) = WebViewExt::settings(&preview) {
            settings.set_enable_javascript_markup(false);
        }

        let paned = gtk::Paned::builder()
            .orientation(gtk::Orientation::Horizontal)
            .start_child(&editor)
            .end_child(&preview)
            .shrink_start_child(false)
            .shrink_end_child(false)
            .position(600)
            .build();
        let page = win.tabs.append(&paned);

        let doc = Rc::new(Doc {
            window: win.window.clone(),
            title: win.title.clone(),
            toasts: win.toasts.clone(),
            page,
            editor,
            view,
            buffer,
            preview,
            file: RefCell::default(),
            pending: RefCell::default(),
        });
        doc.set_mode(&win.modes.active_name().unwrap_or_default());
        doc.apply_dark(adw::StyleManager::default().is_dark());
        doc.connect_signals();
        doc.update_title();
        doc.reload_preview();
        doc
    }

    fn connect_signals(self: &Rc<Self>) {
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
    }

    fn set_mode(&self, mode: &str) {
        self.editor.set_visible(mode != "preview");
        self.preview.set_visible(mode != "editor");
    }

    fn focus(&self) {
        self.view.grab_focus();
    }

    fn apply_dark(&self, dark: bool) {
        let (name, fallback) = if dark { ("downmark-dark", "Adwaita-dark") } else { ("downmark", "Adwaita") };
        let schemes = sourceview5::StyleSchemeManager::default();
        self.buffer.set_style_scheme(schemes.scheme(name).or_else(|| schemes.scheme(fallback)).as_ref());
    }

    fn text(&self) -> String {
        self.buffer.text(&self.buffer.start_iter(), &self.buffer.end_iter(), false).into()
    }

    fn file_name(&self) -> Option<String> {
        self.file.borrow().as_ref().and_then(|f| f.basename()).map(|p| p.to_string_lossy().into_owned())
    }

    fn update_title(&self) {
        let title = title(self.file_name().as_deref(), self.buffer.is_modified());
        self.page.set_title(&title);
        if !self.page.is_selected() {
            return;
        }
        let dir = self.file.borrow().as_ref().and_then(|f| f.parent()).and_then(|p| p.path());
        self.title.set_title(&title);
        self.title.set_subtitle(&dir.map(|d| display_dir(&d, &glib::home_dir())).unwrap_or_default());
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
        if let Some(text) = render::copied_text(&uri) {
            self.window.clipboard().set_text(&text);
            self.toasts.add_toast(adw::Toast::builder().title("Copied").timeout(1).build());
        } else if is_launchable(&uri) {
            gtk::UriLauncher::new(&uri).launch(Some(&self.window), None::<&gio::Cancellable>, |_| {});
        }
        true
    }

    fn is_pristine(&self) -> bool {
        self.file.borrow().is_none() && !self.buffer.is_modified() && self.buffer.char_count() == 0
    }

    fn save(self: &Rc<Self>, after: impl FnOnce(&Rc<Self>) + 'static) {
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

    fn save_as(self: &Rc<Self>, after: impl FnOnce(&Rc<Self>) + 'static) {
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

    fn open(&self, file: &gio::File) -> bool {
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
                true
            }
            Err(err) => {
                let name = file.basename().map(|p| p.display().to_string()).unwrap_or_default();
                self.toast(&format!("Could not open {name}: {err}"));
                false
            }
        }
    }
}

fn main_menu() -> gio::Menu {
    let files = gio::Menu::new();
    files.append(Some("New Document"), Some("win.new"));
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

pub fn display_dir(dir: &std::path::Path, home: &std::path::Path) -> String {
    match dir.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => dir.display().to_string(),
    }
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
    fn display_dir_shortens_home() {
        let home = std::path::Path::new("/home/user");
        assert_eq!(display_dir(std::path::Path::new("/home/user/notes"), home), "~/notes");
        assert_eq!(display_dir(std::path::Path::new("/home/user"), home), "~");
        assert_eq!(display_dir(std::path::Path::new("/home/username"), home), "/home/username");
        assert_eq!(display_dir(std::path::Path::new("/etc"), home), "/etc");
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
