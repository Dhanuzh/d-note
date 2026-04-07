# d-note

A terminal sticky note that lives in the **top-right corner** of your terminal.  
Notes are stored in plain Markdown (`~/notes.md`). Supports checkboxes.

## Install

```
cargo install d-note
```

## Run

```
d-note
```

To keep your terminal free while d-note floats above it, use the `dnote` wrapper (see below).

## Floating popup (recommended)

The `dnote` wrapper opens d-note as a **floating overlay** so your terminal stays usable underneath.

**Install the wrapper:**

```bash
curl -fsSL https://raw.githubusercontent.com/YOUR_USERNAME/d-note/main/scripts/dnote \
  -o ~/.local/bin/dnote && chmod +x ~/.local/bin/dnote
```

**Then use `dnote` instead of `d-note`.**

It auto-detects your environment:

| Environment | Behavior |
|---|---|
| tmux | Floating popup (top-right) |
| kitty | Native overlay (`launch --type=overlay`) |
| zellij | Native floating pane |
| wezterm | New window |
| xterm / X11 | New window anchored top-right |
| alacritty, gnome-terminal, konsole | New window |

**tmux keybinding** — open d-note with `Prefix + n`:

```bash
echo "bind n run-shell 'tmux popup -x #{window_width} -y 0 -w 52 -h 34 -E d-note'" >> ~/.tmux.conf
tmux source-file ~/.tmux.conf
```

## Keys

| Key | Action |
|---|---|
| `a` | Add note |
| `e` | Edit note |
| `d` | Delete note |
| `↑ / ↓` or `j / k` | Navigate |
| `Enter` | View note |
| `Space` | Toggle checkbox (in view) |
| `h` | Hide panel |
| `Ctrl+Space` | Show / hide from anywhere |
| `Ctrl+S` | Save while editing |
| `?` | Help |
| `q / Esc` | Quit |

## Checkboxes

In the note body, use standard Markdown task syntax:

```
- [ ] pending task
- [x] completed task
```

The list view shows a progress badge like `[2/4]`.

## Notes file

`~/notes.md` — plain Markdown, edit it directly with any editor.
