# downmark — Task List

See `tasks/plan.md` for decisions and risks.

## Task 1: Window with live split view

**Description:** Cargo project. Adw.Application with one window: header bar, Gtk.Paned with a sourceview5 View (Markdown language, line numbers) on the left and a webkit6 WebView on the right. WebView loads an HTML shell once; text changes (debounced ~50 ms) render via `render.rs` (pulldown-cmark, GFM options) and replace the body with `evaluate_javascript`.

**Acceptance criteria:**
- [ ] `cargo run` opens the window
- [ ] Typing `# Hi` shows a rendered heading with no visible delay
- [ ] Tables, task lists, fenced code render; preview keeps its scroll position while typing
- [ ] Typing in a 1 MB generated Markdown file shows no perceptible lag

**Verification:**
- [x] Tests pass: `cargo test` (render.rs: heading, table, task list, fenced code)
- [x] Build succeeds: `cargo build --release`, `cargo clippy` clean
- [ ] Manual check: type Markdown, preview follows; follows system dark mode; 1 MB file test

**Dependencies:** None

**Files likely touched:** `Cargo.toml`, `Cargo.lock`, `src/main.rs`, `src/render.rs`, `.gitignore`

**Estimated scope:** M

---

## Task 2: View-mode toggle buttons

**Description:** Adw.ToggleGroup with three icon buttons (Editor / Split / Preview) in the header bar. Editor hides preview, Preview hides editor, Split shows both. Shortcuts Ctrl+1/2/3.

**Acceptance criteria:**
- [ ] Each button shows exactly the expected pane(s)
- [ ] Ctrl+1/2/3 switch modes
- [ ] Returning to Split restores a sensible divider position

**Verification:**
- [x] Build succeeds: `cargo build`, `cargo clippy` clean
- [ ] Manual check: click through all modes and shortcuts

**Dependencies:** Task 1

**Files likely touched:** `src/main.rs`

**Estimated scope:** S

---

## Checkpoint: Skeleton
- [ ] `cargo test` passes
- [ ] Window, live preview and all three modes work
- [ ] Review with human before proceeding

---

## Task 3: Open a file

**Description:** Ctrl+O opens a Gtk.FileDialog filtered to Markdown; `downmark file.md` opens a file from the CLI (Gio `HANDLES_OPEN`). Window title shows the file name.

**Acceptance criteria:**
- [ ] Ctrl+O loads a file into editor and preview
- [ ] `cargo run -- README.md` opens that file
- [ ] Unreadable/non-UTF-8 file shows an error toast, no crash

**Verification:**
- [ ] Manual check: open this repo's README via dialog and CLI; try a binary file

**Dependencies:** Task 1

**Files likely touched:** `src/main.rs`

**Estimated scope:** S

---

## Task 4: Save / Save As, dirty state, close guard

**Description:** Ctrl+S saves (asks for path if new), Ctrl+Shift+S saves as. Title shows `•` when modified. Closing with unsaved changes asks Save / Discard / Cancel (Adw.AlertDialog). Ctrl+N new document.

**Acceptance criteria:**
- [ ] Edits mark the document dirty; saving clears it
- [ ] Closing a dirty document never silently loses data
- [ ] Write failure shows an error and keeps the document dirty

**Verification:**
- [ ] Manual check: edit, close → dialog; save, reopen → content intact; save to read-only dir → error

**Dependencies:** Task 3

**Files likely touched:** `src/main.rs`

**Estimated scope:** M

---

## Task 5: Safe preview

**Description:** WebKit settings: `enable-javascript-markup=false`, `enable-javascript=true`. `decide-policy`: link clicks open in the default browser via `gtk::UriLauncher`, never inside the preview. Base URI set to the file's directory so relative images load.

**Acceptance criteria:**
- [ ] `<script>document.body.innerHTML='pwned'</script>` in the document does nothing
- [ ] Clicking a link opens the system browser
- [ ] `![](img.png)` next to the file renders

**Verification:**
- [ ] Manual check with a test document covering all three

**Dependencies:** Task 3

**Files likely touched:** `src/main.rs`

**Estimated scope:** S

---

## Checkpoint: Usable editor
- [ ] `cargo test` passes, clippy clean
- [ ] Open → edit → save → reopen works with a real README
- [ ] Review with human before proceeding

---

## Task 6: Desktop integration

**Description:** `data/io.github.GreisMicha.downmark.desktop` (MimeType=text/markdown), SVG icon, metainfo XML. README gets install/usage section and screenshot.

**Acceptance criteria:**
- [ ] Installed app appears in the launcher with icon
- [ ] "Open With → downmark" works for `.md` files
- [ ] `desktop-file-validate` and `appstreamcli validate` pass

**Verification:**
- [ ] Build succeeds: `cargo build --release`
- [ ] Manual check: install binary + data files into `~/.local`, launch from menu

**Dependencies:** Tasks 1–5

**Files likely touched:** `data/*.desktop`, `data/*.svg`, `data/*.metainfo.xml`, `README.md`

**Estimated scope:** M

---

## Task 7: PKGBUILD, GitHub release, AUR

**Description:** PKGBUILD (`arch=(x86_64)`, depends: gtk4, libadwaita, gtksourceview5, webkitgtk-6.0; makedepends: cargo) building from the GitHub release tarball with `cargo build --release --locked`. Tag `v0.1.0`, create release, push to `aur.archlinux.org/downmark.git` with `.SRCINFO`.

**Acceptance criteria:**
- [ ] `makepkg -si` in a clean dir installs and runs the app
- [ ] `namcap` reports no errors on PKGBUILD and package
- [ ] Package live on AUR

**Verification:**
- [ ] Build succeeds: `makepkg -f && namcap PKGBUILD *.pkg.tar.zst`
- [ ] Manual check: `yay -S downmark` on this machine

**Dependencies:** Task 6

**Files likely touched:** `PKGBUILD`, `.SRCINFO` (in the AUR repo)

**Estimated scope:** S

---

## Checkpoint: Released
- [ ] All acceptance criteria met
- [ ] Installable from AUR, launches from the app menu

## Later (not in v1)
- Synced scrolling editor ↔ preview
- Remember view mode / window size
- Export to HTML/PDF
- Tabs / multiple documents
