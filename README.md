# downmark

A lightweight Markdown editor and renderer with split view, built for Linux with GTK 4 and libadwaita.

![downmark](data/screenshot.png)

## Features

- Editor, Split and Preview modes, switched from the header bar
- Live preview while typing, first line aligned with the editor
- GitHub flavored Markdown: tables, task lists, footnotes, strikethrough
- Export the rendered document as PDF or PNG
- Light, Dark or System theme
- Rebindable keyboard shortcuts
- Scripts in documents never run; links open in your browser

## Install

### Arch Linux (AUR)

```sh
yay -S downmark
```

### From source

Dependencies: `rust`, `gtk4`, `libadwaita`, `gtksourceview5`, `webkitgtk-6.0`.

```sh
make
sudo make install            # into /usr/local
make install PREFIX=~/.local # or only for your user
```

`make uninstall` (with the same `PREFIX`) removes it again.

## Default app for Markdown

After installing, downmark shows up in GNOME's app grid and in **Open With…** for `.md` files.
To make it the default, pick it in **Open With… → Always use for this file type**, or run:

```sh
xdg-mime default io.github.GreisMicha.downmark.desktop text/markdown
```

## Keyboard shortcuts

| Action | Default |
|---|---|
| New window | Ctrl+N |
| Open | Ctrl+O |
| Save | Ctrl+S |
| Save as | Shift+Ctrl+S |
| Close window | Ctrl+W |
| Editor / Split / Preview | Ctrl+1 / Ctrl+2 / Ctrl+3 |
| Export as PDF | Ctrl+E |
| Export as PNG | Shift+Ctrl+E |
| Settings | Ctrl+, |

All shortcuts can be changed in **Settings → Shortcuts**. Settings are stored in `~/.config/downmark/settings.ini`.

## License

MIT
