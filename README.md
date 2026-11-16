# doer-tui

<p align="center">
  <strong>A fast, keyboard-driven terminal user interface (TUI) for the <a href="https://github.com/leouofa/doer">Doer</a> task and checklist platform.</strong>
</p>

<p align="center">
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-2021_Edition-dea584?style=flat-square&logo=rust&logoColor=white" alt="Rust 2021"></a>
  <a href="https://ratatui.rs"><img src="https://img.shields.io/badge/Ratatui-0.28-111111?style=flat-square" alt="Ratatui"></a>
  <a href="https://github.com/crossterm-rs/crossterm"><img src="https://img.shields.io/badge/Crossterm-0.28-blue?style=flat-square" alt="Crossterm"></a>
  <img src="https://img.shields.io/badge/Architecture-Local--First-22c55e?style=flat-square" alt="Local-First Architecture">
  <a href="https://github.com/leouofa/doer"><img src="https://img.shields.io/badge/Doer-Rails_8-D30001?style=flat-square&logo=rubyonrails&logoColor=white" alt="Doer Engine"></a>
  <a href="https://github.com/leouofa/doer-mcp"><img src="https://img.shields.io/badge/MCP_Server-doer--mcp-6366f1?style=flat-square&logo=github&logoColor=white" alt="Doer MCP Server"></a>
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-blue?style=flat-square" alt="License: MIT"></a>
</p>

---

A fast, keyboard-driven terminal user interface (TUI) for the [Doer](https://github.com/leouofa/doer) task and checklist platform built with [Ratatui](https://ratatui.rs) and [Crossterm](https://github.com/crossterm-rs/crossterm).

## Features

- **Host URL Resolution & Host Switcher**:
  - **Environment default**: When `DOER_REMOTE_URL` is set in the shell (e.g. `export DOER_REMOTE_URL="http://your-host:3010"`), running `cargo run` defaults directly to that URL.
  - **Fallback**: If `DOER_REMOTE_URL` is not set, defaults to `http://localhost:3000`.
  - **CLI overrides**: `--url <URL>`, `--host <URL>`, and `--local` (`http://localhost:3000`) always take precedence over the environment.
  - **Locking behavior**:
    - If `DOER_REMOTE_URL` is set, host switching is locked by default so the TUI stays pinned to your configured instance.
    - Pass `--allow-switch` to re-enable the Host Switcher dialog while using `DOER_REMOTE_URL`.
    - If `DOER_REMOTE_URL` is not set, Host Switcher (`H` or `u`) is enabled automatically.
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

> [!NOTE]
> `doer-tui` connects to a running [Doer](https://github.com/leouofa/doer) backend instance (defaults to `http://localhost:3000` or `DOER_REMOTE_URL`).

### Install via Cargo (crates.io)

```bash
cargo install the-doer-tui

# Launch the app
doer-tui
```

### Build from Source

```bash
# Build
cargo build --release

# Run (defaults to DOER_REMOTE_URL if set, otherwise http://localhost:3000)
cargo run

# Force local host even if DOER_REMOTE_URL is set
cargo run -- --local

# Specify any target host explicitly
cargo run -- --url http://localhost:3010

# Allow in-app Host Switcher even when DOER_REMOTE_URL is set
cargo run -- --allow-switch
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
| `H` / `u` | Open Host Switcher dialog (when enabled or via `--allow-switch`) |
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
