# downmark — Task List

See `tasks/plan.md` for decisions and risks. Test command: `cargo test`. Lint: `cargo clippy --all-targets -- -D warnings`.

## Done
- [x] Task 1: Window with live split view
- [x] Task 2: View-mode toggle buttons

---

## Task 3: Preview line 1 aligned, no tooltips

**Description:** Preview body top padding matches the editor top margin; first block has no top margin. Remove tooltips from mode buttons.

**Acceptance criteria:**
- [x] First line of editor and preview start at the same height
- [ ] Hovering mode buttons shows no tooltip

**Verification:**
- [x] Tests pass: `cargo test` (page CSS has matching top padding and first-child margin reset)
- [x] Manual check: open a file starting with `# Title`

**Dependencies:** None · **Files:** `src/render.rs`, `src/main.rs` · **Scope:** XS

---

## Task 4: Open a file

**Description:** Ctrl+O file dialog (Markdown filter) and `downmark file.md` (HANDLES_OPEN). Title shows file name. Non-UTF-8 / unreadable files show an error toast.

**Acceptance criteria:**
- [x] Ctrl+O and CLI argument load the file into editor and preview
- [x] Binary file shows an error toast, no crash

**Verification:**
- [x] Tests pass: `cargo test` (window title formatting, UTF-8 decode error)
- [x] Manual check: `cargo run -- README.md`

**Dependencies:** None · **Files:** `src/main.rs`, `src/window.rs` · **Scope:** M

---

## Task 5: Save, Save As, New, close guard

**Description:** Ctrl+S overwrites the opened file (Save As dialog if untitled), Ctrl+Shift+S Save As, Ctrl+N new window. Title shows `•` when modified. Closing a modified document asks Save / Discard / Cancel.

**Acceptance criteria:**
- [x] Ctrl+S writes the file, clears `•`
- [ ] Closing modified document never silently loses data
- [x] Write failure shows a toast and keeps document modified

**Verification:**
- [x] Tests pass: `cargo test` (save writes exact bytes to disk, title shows modified marker)
- [ ] Manual check: edit → Ctrl+S → `cat` file

**Dependencies:** Task 4 · **Files:** `src/window.rs`, `src/main.rs` · **Scope:** M

---

## Task 6: Safe preview

**Description:** Clicked links open in the default browser, never inside the preview. Page base URI is the file's directory so relative images load. Document scripts never run.

**Acceptance criteria:**
- [ ] Link click opens system browser
- [x] `![](img.png)` next to the file renders
- [x] `<img src=x onerror=...>` and `<script>` do nothing

**Verification:**
- [x] Tests pass: `cargo test` (base URI from file path)
- [x] Manual check with test document

**Dependencies:** Task 4 · **Files:** `src/window.rs`, `src/render.rs` · **Scope:** S

---

## Checkpoint: Usable editor
- [x] Tests pass, clippy clean
- [x] Open → edit → Ctrl+S → reopen works

---

## Task 7: Settings storage and shortcut table

**Description:** `settings.rs`: theme + shortcut map, defaults, load/save via `glib::KeyFile`, conflict detection. App applies accels and theme from settings on startup.

**Acceptance criteria:**
- [x] Missing/garbage settings file falls back to defaults
- [x] Settings round-trip through the file
- [x] Assigning an accel already used by another action is reported as a conflict

**Verification:**
- [x] Tests pass: `cargo test` (defaults, round-trip, invalid theme, conflict)

**Dependencies:** None · **Files:** `src/settings.rs`, `src/main.rs` · **Scope:** S

---

## Task 8: Settings dialog — theme and shortcuts

**Description:** Header bar menu button opens `Adw.PreferencesDialog` (also Ctrl+,). Appearance: System / Light / Dark. Shortcuts: every action with its current shortcut; click to record a new one (Esc cancels), conflicts rejected with a toast, "Reset to defaults". Changes apply instantly and persist.

**Acceptance criteria:**
- [x] Theme change applies to editor and preview immediately and persists
- [ ] Rebinding a shortcut works immediately and persists across restart
- [x] All shortcuts are listed with their current binding

**Verification:**
- [ ] Manual check: change theme, rebind Ctrl+2 → Ctrl+Shift+2, restart

**Dependencies:** Task 7 · **Files:** `src/prefs.rs`, `src/main.rs`, `src/window.rs` · **Scope:** M

---

## Task 9: Export PDF and PNG

**Description:** Settings dialog "Export" group with "Export as PDF…" and "Export as PNG…" (also bound to shortcuts). PNG = full rendered document snapshot; PDF = WebKit print to file.

**Acceptance criteria:**
- [x] PNG contains the whole rendered document
- [x] PDF has selectable text
- [x] Failure shows a toast

**Verification:**
- [x] Tests pass: `cargo test` (export file name suggestion)
- [x] Manual check: export README, open PNG and PDF

**Dependencies:** Tasks 4, 8 · **Files:** `src/window.rs`, `src/prefs.rs` · **Scope:** M

---

## Checkpoint: Feature complete
- [x] Tests pass, clippy clean
- [x] Settings, export and shortcuts work end-to-end

---

## Task 10: Desktop integration

**Description:** `.desktop` (MimeType=text/markdown), SVG icon, metainfo, README install/usage/shortcuts.

**Acceptance criteria:**
- [x] `desktop-file-validate` and `appstreamcli validate --no-net` pass

**Dependencies:** Tasks 3–9 · **Files:** `data/*`, `README.md` · **Scope:** S

---

## Task 11: PKGBUILD and release

**Description:** `packaging/PKGBUILD` (+ `.SRCINFO`) building from the GitHub release tarball. Tag `v0.1.0`, GitHub Actions publishes the release with binary tarball and notes.

**Acceptance criteria:**
- [x] GitHub release v0.1.0 exists with binary tarball and notes
- [x] `makepkg` builds and installs from the release tarball
- [ ] AUR push (needs user's AUR SSH key)

**Dependencies:** Task 10 · **Files:** `packaging/PKGBUILD`, `packaging/.SRCINFO`, `.github/workflows/release.yml` · **Scope:** S

---

## Checkpoint: Released
- [x] Release live on GitHub, PKGBUILD verified

## Later (not in v1)
- Synced scrolling beyond line 1
- Tabs / multiple documents per window
- Remember window size
