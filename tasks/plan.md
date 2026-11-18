# Implementation Plan: downmark

Source spec: none (README + user requests)

## Overview
downmark is a lightweight Linux Markdown editor/renderer. One window per document, three view modes (Editor / Split / Preview) switched by header bar buttons like PhpStorm. Live preview without lag. Settings dialog for theme, export (PDF/PNG) and rebindable shortcuts. Released via GitHub Actions; AUR package prepared.

## Architecture Decisions
- **Rust + gtk4-rs + libadwaita**, sourceview5 editor, webkit6 preview, pulldown-cmark rendering.
- **Preview updates** replace `document.body.innerHTML` via `evaluate_javascript`; page reloaded only when the base directory changes (file opened/saved elsewhere) so relative images resolve.
- **Line 1 alignment:** preview body top padding equals editor top margin and the first block has no top margin, so the first line starts at the same height. Later drift is accepted.
- **Settings** stored with `glib::KeyFile` in `~/.config/downmark/settings.ini` (theme + shortcuts). No GSettings schema, so `cargo run` works without installing anything.
- **Shortcuts** live in one table (id, action, title, default). Applied with `set_accels_for_action`, rebound in the settings dialog, conflicts rejected.
- **No tooltips** on the mode buttons; shortcuts are listed in settings.
- **Export:** PNG via `WebView::snapshot` (full document) → `Texture::save_to_png`. PDF via `webkit6::PrintOperation` with print-to-file settings (vector text, no dialog).
- **Theme:** System / Light / Dark through `adw::StyleManager::set_color_scheme`; editor scheme and preview `color-scheme` follow `is_dark`.
- **Files:** `gio::File` load/replace; dirty state is `TextBuffer::modified`. Ctrl+N opens a new window, so no "discard current?" prompt is needed.
- **Layout:** `main.rs` (app, actions, accels), `window.rs` (window, files, preview, export), `prefs.rs` (settings dialog), `settings.rs` (pure, tested), `render.rs` (pure, tested).

## Dependency Graph
```
render.rs ─► window (preview, line-1 alignment)
settings.rs ─► shortcuts table ─► accels ─► prefs dialog (theme, shortcuts, export buttons)
window: open ─► save / save-as / close guard ─► safe preview (links, base URI)
                                      └──► export PDF / PNG
all ─► desktop files ─► PKGBUILD ─► v0.1.0 tag ─► GitHub release
```

## Task List
See `tasks/todo.md`.

## Risks and Mitigations
| Risk | Impact | Mitigation |
|------|--------|------------|
| No AUR SSH key registered for this machine | High | Ship PKGBUILD in `packaging/`, verified with `makepkg`; user registers key and pushes. |
| Arch `extra` requires a Package Maintainer to adopt the package | High | AUR first. |
| Snapshot of a hidden WebView (Editor mode) may fail | Med | Show error toast; switch to Split before exporting if needed. |
| PrintOperation PDF output depends on GTK print-to-file backend | Med | `gtk4` ships it; verify by exporting a real file. |
| GNOME Wayland blocks automated screenshots | Low | Verify logic by tests and exported PNGs; user does the final visual check. |
