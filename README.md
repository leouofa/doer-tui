# doer-tui

A fast, keyboard-driven terminal user interface (TUI) for the **Doer** task and checklist platform built with [Ratatui](https://ratatui.rs) and [Crossterm](https://github.com/crossterm-rs/crossterm).

## Features

- **Configurable Host URL & Quick Host Switcher**:
  - Priority: CLI `--url <URL>` / `--host <URL>` / `--local` / `--remote` → `DOER_API_URL` env var → default `http://localhost:3000`.
  - Built-in presets for local instances (`http://localhost:3000` and `http://localhost:3010`), plus support for custom remote URLs via `DOER_REMOTE_URL`.
  - Press `H` or `u` at any time to open the in-app **Host Switcher** modal to toggle between presets or enter any custom URL on the fly.
- **3-Pane Master–Detail Layout**:
  - **Header**: Live host URL badge, stats counters (Total, Pinned, Projects, Attachments), and search query.
  - **Pane 1 `[1] Scopes & Projects`**: All Todo Lists, ★ Pinned, Standalone, and all Projects with task counts.
  - **Pane 2 `[2] Todo Lists`**: Filtered list showing pin markers (`★`), titles, checklist progress badges (`[done/total]`), project tags, and updated dates.
  - **Pane 3 `[3] Detail & Checklist`**: Metadata header, attachment counters, and rich Markdown body with interactive `- [ ]` / `- [x]` checklist lines.
- **Interactive Checklist Navigation & Toggling**:
  - `j` / `k` in Pane 3 moves the checklist cursor directly between `- [ ]` and `- [x]` items, automatically scrolling the document to keep the active task in view.
  - `Space` or `Enter` toggles the active item between `[ ]` and `[x]`, instantly persisting the updated Markdown back to the server.
  - `J` / `K` or `PageDown` / `PageUp` / `g` / `G` scrolls the full document view.
- **Full CRUD & Modals**:
  - `a`: Quick Add Task modal (`- [ ] <task>`) with optional section header (appends to Markdown body).
  - `n` / `e`: Create or edit Projects (in Pane 1) or Todo Lists (in Pane 2/3).
  - `d`: Delete Project, Todo List, or Attachment with confirmation dialog.
  - `p`: Toggle pin status on selected Todo List.
  - `K` / `J`: Reorder pinned Todo Lists when viewing the Pinned scope.
  - `E`: Open full Markdown body in external `$EDITOR` (e.g. `nvim`, `nano`, `vi`). Suspends TUI cleanly and reloads upon exit.
- **Attachments & Clipboard**:
  - `A`: Open Attachment Manager modal to view attachments, upload new files by path (`u`), download files to disk (`s`), copy attachment URLs (`y`), or delete files (`d`).
  - `y`: Yank active task text (or title) to system clipboard (`arboard`).
  - `Y`: Yank entire Markdown body to system clipboard.
- **Live Search**:
  - `/`: Open real-time search filter across Todo Lists. Press `Esc` to clear.

## Installation & Running

```bash
# Build
cargo build --release

# Run with default host (http://localhost:3000)
cargo run

# Run pointing to a secondary/remote host preset
cargo run -- --remote

# Or specify any host URL directly
cargo run -- --url http://localhost:3010

# Or via environment variable
export DOER_API_URL="http://localhost:3010"
cargo run
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `1`, `2`, `3` | Direct focus to Pane 1 (Sidebar), 2 (Todo Lists), 3 (Detail) |
| `Tab` / `BackTab` | Cycle focus forward / backward between panes |
| `h` / `l` | Move focus left / right |
| `j` / `k` | Move selection down / up in focused pane |
| `Enter` | Enter pane 2 from sidebar, or pane 3 from todo list |
| `Space` / `Enter` | Toggle active checklist item `[ ]` ⇄ `[x]` (in Pane 3) |
| `a` | Quick-add new task (`- [ ]`) to current list |
| `E` | Open markdown body in `$EDITOR` |
| `J` / `K` | Scroll detail document down / up |
| `g` / `G` | Jump to top / bottom of detail document |
| `y` | Yank active task text to system clipboard |
| `Y` | Yank full markdown body to system clipboard |
| `H` / `u` | Open Host Switcher dialog |
| `/` | Live search & filter todo lists (`Esc` to clear) |
| `n` | New Project (in pane 1) or Todo List (in pane 2/3) |
| `e` | Edit Project or Todo List metadata |
| `d` | Delete with confirmation |
| `p` | Toggle pin on selected Todo List |
| `K` / `J` | Reorder pinned items up / down (in Pinned scope) |
| `A` | Manage attachments |
| `r` | Refresh data from server |
| `?` | Show help modal |
| `q` | Quit application |
