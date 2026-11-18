# Implementation Plan: downmark

Source spec: none (README + user request)

## Overview
downmark is a lightweight Linux Markdown editor/renderer. One window, one document, three view modes switched by three toggle buttons in the header bar, like PhpStorm: **Editor**, **Split**, **Preview**. Preview updates live while typing without lag. Distributed via AUR (Arch `extra` is not something we can upload to; see Risks), hosted on GitHub under MIT.

## Architecture Decisions
- **Rust + gtk4-rs + libadwaita.** Native speed for Markdown rendering, fast startup, low memory. Target hardware: Framework 16 / Radeon 780M; goal is zero perceptible typing lag.
- **sourceview5** crate (GtkSourceView 5) for the editor: Markdown highlighting, line numbers, undo, dark-mode style schemes.
- **webkit6** crate (WebKitGTK 6.0) for the preview.
- **pulldown-cmark** for Markdown → HTML with GFM options (tables, task lists, strikethrough, footnotes). Sub-millisecond for typical documents, so rendering runs on the UI thread with no worker thread.
- **Preview update without reload:** load an HTML shell once, then replace the content via `evaluate_javascript("document.body.innerHTML = …")`. No flicker, scroll position kept.
- **Preview security:** `enable-javascript-markup=false` (document scripts never run) while `enable-javascript=true` (our `evaluate_javascript` still works). Navigation blocked; links open in the default browser.
- **Debounce ~50 ms** with `glib::timeout_add_local_once`, reset on every change.
- **Gtk.Paned + Adw.ToggleGroup** (libadwaita ≥ 1.7, crate feature `v1_7`) for the three modes.
- **Layout:** `src/main.rs` (app + window), `src/render.rs` (markdown → HTML, pure, unit-tested). Split only if a file passes ~400 lines.
- **Packaging:** Cargo + `.desktop` + SVG icon + metainfo. PKGBUILD `arch=(x86_64)`, `makedepends=(cargo)`, builds with `--locked`.

## Dependency Graph
```
render.rs (md → html, pure, testable)
    │
window: editor ──► preview (shell loaded once, innerHTML updates)
    │
view-mode toggle (Editor / Split / Preview)
    │
file open / save / dirty state
    │
packaging (.desktop, icon, metainfo, PKGBUILD) ──► AUR release
```

## Task List

### Phase 1: Walking skeleton
- [ ] Task 1: Window with live split view
- [ ] Task 2: View-mode toggle buttons

### Checkpoint: Skeleton
- [ ] `cargo run` opens a window, typing updates preview without lag on a 1 MB file, all three modes work

### Phase 2: Core features
- [ ] Task 3: Open a file (dialog + CLI argument)
- [ ] Task 4: Save / Save As with dirty indicator and close guard
- [ ] Task 5: Safe preview (no document JS, external links in browser, relative images)

### Checkpoint: Usable editor
- [ ] Open, edit, save a real README end-to-end; no data loss on close

### Phase 3: Ship
- [ ] Task 6: Desktop integration (.desktop, icon, metainfo, README)
- [ ] Task 7: PKGBUILD + first GitHub release + AUR upload

### Checkpoint: Released
- [ ] `yay -S downmark` installs it and it launches from the app menu

## Risks and Mitigations
| Risk | Impact | Mitigation |
|------|--------|------------|
| Arch `extra` only accepts packages adopted by a Package Maintainer; we cannot upload there | High | Ship to AUR. `extra` may follow once it has votes and a maintainer picks it up. |
| gtk4-rs callback/clone boilerplate makes code verbose | Med | Keep state in one `Rc<…>` struct per window; use `glib::clone!`. |
| Passing large HTML into `evaluate_javascript` needs escaping | Med | Encode the HTML as a JSON string literal (`serde_json::to_string`) — no hand-written escaping. |
| WebKitGTK is the heaviest part (RAM, startup) | Med | Accept it; create the WebView once and reuse it. Still far lighter than Electron. |
| Crate versions must match system libs (gtk4 4.24, libadwaita 1.10, webkitgtk-6.0 2.54) | Low | Pin crate features to what Arch ships; `Cargo.lock` committed. |

## Decisions Still Open
- Should the view mode be remembered between launches, or always start in Split? (Default: always Split.)
- Synced scrolling between editor and preview — planned for later.
- App ID: `io.github.GreisMicha.downmark` (default unless changed).
