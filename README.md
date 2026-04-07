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
