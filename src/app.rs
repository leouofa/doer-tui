#![allow(dead_code)]

use std::path::PathBuf;

use anyhow::Result;
use arboard::Clipboard;
use crossterm::event::KeyCode;

use crate::api::ApiClient;
use crate::checklist::{
    append_to_checklist, parse_checklist_items, toggle_checklist_item_by_index,
    ChecklistItem,
};
use crate::config::{get_preset_hosts, Config};
use crate::models::{
    Project, ProjectInput, Stats, TodoList, TodoListDetail, TodoListInput,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Sidebar,
    TodoLists,
    Detail,
}

impl Pane {
    pub fn next(self) -> Self {
        match self {
            Pane::Sidebar => Pane::TodoLists,
            Pane::TodoLists => Pane::Detail,
            Pane::Detail => Pane::Sidebar,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Pane::Sidebar => Pane::Detail,
            Pane::TodoLists => Pane::Sidebar,
            Pane::Detail => Pane::TodoLists,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeItem {
    All,
    Pinned,
    Standalone,
    Project(Project),
}

#[derive(Debug, Clone)]
pub enum DeleteTarget {
    Project(Project),
    TodoList(TodoList),
    Attachment {
        todo_list_id: u64,
        attachment_id: u64,
        filename: String,
    },
}

#[derive(Debug, Clone)]
pub enum Modal {
    HostSwitcher {
        cursor: usize,
        custom_input: String,
        entering_custom: bool,
    },
    Search {
        input: String,
    },
    QuickAddTask {
        task_input: String,
        section_input: String,
        focus_section: bool,
    },
    ProjectForm {
        id: Option<u64>,
        name_input: String,
        desc_input: String,
        focus_desc: bool,
    },
    TodoListForm {
        id: Option<u64>,
        title_input: String,
        project_choice_idx: usize, // 0 = None (Standalone), 1.. = index into projects
        pinned: bool,
        focus_field: usize, // 0: title, 1: project, 2: pinned
    },
    ConfirmDelete {
        target: DeleteTarget,
    },
    Attachments {
        cursor: usize,
        upload_input: Option<String>,
        dest_input: Option<String>,
    },
    Help,
}

pub struct App {
    pub config: Config,
    pub client: ApiClient,
    pub should_quit: bool,
    pub pending_editor: Option<(u64, PathBuf)>, // todo_list_id, temp_path
    pub status: Option<(String, bool)>,         // (message, is_error)

    // Layout & Navigation
    pub focused_pane: Pane,
    pub modal: Option<Modal>,

    // Data
    pub stats: Stats,
    pub preset_hosts: Vec<String>,
    pub projects: Vec<Project>,
    pub todo_lists: Vec<TodoList>,
    pub current_todo_list: Option<TodoListDetail>,
    pub checklist_items: Vec<ChecklistItem>,

    // Selections
    pub sidebar_selected: usize,
    pub todo_lists_selected: usize,
    pub detail_selected_checklist: Option<usize>,
    pub detail_scroll: usize,

    // Search query
    pub search_query: Option<String>,

    pub clipboard: Option<Clipboard>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let client = ApiClient::new(&config);
        let clipboard = Clipboard::new().ok();

        let mut app = Self {
            config,
            client,
            should_quit: false,
            pending_editor: None,
            status: None,

            focused_pane: Pane::TodoLists,
            modal: None,

            stats: Stats::default(),
            preset_hosts: get_preset_hosts(),
            projects: Vec::new(),
            todo_lists: Vec::new(),
            current_todo_list: None,
            checklist_items: Vec::new(),

            sidebar_selected: 0,
            todo_lists_selected: 0,
            detail_selected_checklist: None,
            detail_scroll: 0,

            search_query: None,
            clipboard,
        };

        app.reload_all();
        app
    }

    pub fn set_status(&mut self, msg: impl Into<String>, is_error: bool) {
        self.status = Some((msg.into(), is_error));
    }

    pub fn clear_status(&mut self) {
        self.status = None;
    }

    // Host Management
    pub fn switch_host(&mut self, new_url: &str) {
        let normalized = crate::config::normalize_url(new_url);
        self.config.set_url(&normalized);
        self.client.set_base_url(&normalized);
        self.set_status(format!("Switched host to {}", normalized), false);
        self.reload_all();
    }

    // Data Fetching
    pub fn reload_all(&mut self) {
        self.load_stats();
        self.load_projects();
        self.load_todo_lists();
    }

    pub fn load_stats(&mut self) {
        match self.client.get_stats() {
            Ok(s) => self.stats = s,
            Err(e) => self.set_status(format!("Stats error: {}", e), true),
        }
    }

    pub fn load_projects(&mut self) {
        match self.client.list_projects(None) {
            Ok(projs) => {
                self.projects = projs;
                if self.sidebar_selected >= self.total_sidebar_items() {
                    self.sidebar_selected = self.total_sidebar_items().saturating_sub(1);
                }
            }
            Err(e) => self.set_status(format!("Projects error: {}", e), true),
        }
    }

    pub fn total_sidebar_items(&self) -> usize {
        3 + self.projects.len()
    }

    pub fn get_sidebar_item(&self, idx: usize) -> ScopeItem {
        match idx {
            0 => ScopeItem::All,
            1 => ScopeItem::Pinned,
            2 => ScopeItem::Standalone,
            n => {
                if let Some(proj) = self.projects.get(n - 3) {
                    ScopeItem::Project(proj.clone())
                } else {
                    ScopeItem::All
                }
            }
        }
    }

    pub fn load_todo_lists(&mut self) {
        let scope_item = self.get_sidebar_item(self.sidebar_selected);
        let q = self.search_query.as_deref();

        let res = match scope_item {
            ScopeItem::All => self.client.list_todo_lists(None, None, None, q),
            ScopeItem::Pinned => self.client.list_todo_lists(None, None, Some(true), q),
            ScopeItem::Standalone => {
                self.client.list_todo_lists(None, Some("standalone"), None, q)
            }
            ScopeItem::Project(p) => {
                self.client.list_todo_lists(Some(p.id), None, None, q)
            }
        };

        match res {
            Ok(lists) => {
                self.todo_lists = lists;
                if self.todo_lists.is_empty() {
                    self.todo_lists_selected = 0;
                    self.current_todo_list = None;
                    self.checklist_items.clear();
                    self.detail_selected_checklist = None;
                } else {
                    if self.todo_lists_selected >= self.todo_lists.len() {
                        self.todo_lists_selected = self.todo_lists.len() - 1;
                    }
                    self.load_current_todo_list_detail();
                }
            }
            Err(e) => self.set_status(format!("Todo lists error: {}", e), true),
        }
    }

    pub fn load_current_todo_list_detail(&mut self) {
        if let Some(tl) = self.todo_lists.get(self.todo_lists_selected) {
            match self.client.get_todo_list(tl.id) {
                Ok(detail) => {
                    self.checklist_items = parse_checklist_items(detail.body.as_deref().unwrap_or(""));
                    if self.checklist_items.is_empty() {
                        self.detail_selected_checklist = None;
                    } else if let Some(cur) = self.detail_selected_checklist {
                        if cur >= self.checklist_items.len() {
                            self.detail_selected_checklist = Some(self.checklist_items.len() - 1);
                        }
                    } else {
                        self.detail_selected_checklist = Some(0);
                    }
                    self.current_todo_list = Some(detail);
                }
                Err(e) => {
                    self.set_status(format!("Detail load error: {}", e), true);
                    self.current_todo_list = None;
                    self.checklist_items.clear();
                }
            }
        } else {
            self.current_todo_list = None;
            self.checklist_items.clear();
            self.detail_selected_checklist = None;
        }
    }

    // Toggle Checklist Item
    pub fn toggle_current_checklist_item(&mut self) {
        let (detail, cur_idx) = match (&self.current_todo_list, self.detail_selected_checklist) {
            (Some(d), Some(idx)) => (d.clone(), idx),
            _ => return,
        };

        let body = detail.body.as_deref().unwrap_or("");
        if let Some((updated_body, item)) = toggle_checklist_item_by_index(body, cur_idx) {
            let input = TodoListInput {
                title: detail.title.clone(),
                body: Some(updated_body.clone()),
                ..Default::default()
            };

            match self.client.update_todo_list(detail.id, &input) {
                Ok(new_detail) => {
                    // Update in list
                    if let Some(list_item) = self.todo_lists.get_mut(self.todo_lists_selected) {
                        list_item.body = new_detail.body.clone();
                    }
                    self.checklist_items = parse_checklist_items(&updated_body);
                    self.current_todo_list = Some(new_detail);
                    let state_str = if item.completed { "done" } else { "pending" };
                    self.set_status(format!("Task marked {}", state_str), false);
                }
                Err(e) => self.set_status(format!("Failed to update task: {}", e), true),
            }
        }
    }

    // Quick Add Checklist Item
    pub fn append_checklist_item(&mut self, text: &str, section: Option<&str>) {
        let detail = match &self.current_todo_list {
            Some(d) => d.clone(),
            None => return,
        };

        let current_body = detail.body.as_deref().unwrap_or("");
        let updated = append_to_checklist(current_body, &[text.to_string()], section);

        let input = TodoListInput {
            title: detail.title.clone(),
            body: Some(updated.clone()),
            ..Default::default()
        };

        match self.client.update_todo_list(detail.id, &input) {
            Ok(new_detail) => {
                if let Some(list_item) = self.todo_lists.get_mut(self.todo_lists_selected) {
                    list_item.body = new_detail.body.clone();
                }
                self.checklist_items = parse_checklist_items(&updated);
                self.current_todo_list = Some(new_detail);
                self.set_status("Task appended successfully", false);
            }
            Err(e) => self.set_status(format!("Failed to append task: {}", e), true),
        }
    }

    // Toggle Pin on selected Todo List
    pub fn toggle_pin_current(&mut self) {
        if let Some(tl) = self.todo_lists.get(self.todo_lists_selected) {
            match self.client.toggle_pin(tl.id) {
                Ok(resp) => {
                    let msg = if resp.pinned {
                        "Pinned todo list"
                    } else {
                        "Unpinned todo list"
                    };
                    self.set_status(msg, false);
                    self.load_stats();
                    self.load_todo_lists();
                }
                Err(e) => self.set_status(format!("Toggle pin error: {}", e), true),
            }
        }
    }

    // Reorder pinned todo lists up/down
    pub fn move_pinned_current(&mut self, direction_up: bool) {
        let is_pinned_scope = matches!(self.get_sidebar_item(self.sidebar_selected), ScopeItem::Pinned);
        if !is_pinned_scope || self.todo_lists.len() < 2 {
            return;
        }

        let cur_idx = self.todo_lists_selected;
        let new_idx = if direction_up {
            if cur_idx == 0 { return; }
            cur_idx - 1
        } else {
            if cur_idx + 1 >= self.todo_lists.len() { return; }
            cur_idx + 1
        };

        self.todo_lists.swap(cur_idx, new_idx);
        self.todo_lists_selected = new_idx;

        let ids: Vec<u64> = self.todo_lists.iter().map(|tl| tl.id).collect();
        if let Err(e) = self.client.reorder_pinned(&ids) {
            self.set_status(format!("Reorder error: {}", e), true);
        } else {
            self.set_status("Reordered pinned list", false);
        }
    }

    // Yank to clipboard
    pub fn yank_current(&mut self, full_body: bool) {
        let text_to_copy = if full_body {
            self.current_todo_list
                .as_ref()
                .and_then(|d| d.body.clone())
                .unwrap_or_default()
        } else if let Some(idx) = self.detail_selected_checklist {
            self.checklist_items
                .get(idx)
                .map(|item| item.text.clone())
                .unwrap_or_default()
        } else {
            self.current_todo_list
                .as_ref()
                .map(|d| d.title.clone())
                .unwrap_or_default()
        };

        if text_to_copy.is_empty() {
            self.set_status("Nothing to copy", false);
            return;
        }

        if let Some(cb) = &mut self.clipboard {
            if let Err(e) = cb.set_text(text_to_copy.clone()) {
                self.set_status(format!("Clipboard error: {}", e), true);
            } else {
                let preview = if text_to_copy.len() > 30 {
                    format!("{}...", &text_to_copy[..30])
                } else {
                    text_to_copy
                };
                self.set_status(format!("Copied: \"{}\"", preview), false);
            }
        } else {
            self.set_status("Clipboard unavailable", true);
        }
    }

    // External Editor Setup
    pub fn prepare_external_editor(&mut self) -> Result<()> {
        let detail = match &self.current_todo_list {
            Some(d) => d.clone(),
            None => return Ok(()),
        };

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join(format!("doer-todo-{}.md", detail.id));
        std::fs::write(&file_path, detail.body.as_deref().unwrap_or(""))?;

        self.pending_editor = Some((detail.id, file_path));
        Ok(())
    }

    pub fn complete_external_editor(&mut self) -> Result<()> {
        if let Some((id, path)) = self.pending_editor.take() {
            if path.exists() {
                let content = std::fs::read_to_string(&path)?;
                let _ = std::fs::remove_file(&path);

                if let Some(detail) = &self.current_todo_list {
                    if detail.id == id {
                        let input = TodoListInput {
                            title: detail.title.clone(),
                            body: Some(content.clone()),
                            ..Default::default()
                        };
                        match self.client.update_todo_list(id, &input) {
                            Ok(new_detail) => {
                                if let Some(list_item) =
                                    self.todo_lists.get_mut(self.todo_lists_selected)
                                {
                                    list_item.body = new_detail.body.clone();
                                }
                                self.checklist_items = parse_checklist_items(&content);
                                self.current_todo_list = Some(new_detail);
                                self.set_status("Saved markdown from editor", false);
                            }
                            Err(e) => {
                                self.set_status(format!("Failed to save from editor: {}", e), true)
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    // Keyboard Event Handling
    pub fn handle_key(&mut self, key: KeyCode) {
        // If modal open, delegate to modal handler
        if self.modal.is_some() {
            self.handle_modal_key(key);
            return;
        }

        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('?') => self.modal = Some(Modal::Help),
            KeyCode::Char('H') | KeyCode::Char('u') => {
                if self.config.allow_host_switcher {
                    self.modal = Some(Modal::HostSwitcher {
                        cursor: 0,
                        custom_input: self.config.base_url.clone(),
                        entering_custom: false,
                    });
                } else {
                    self.set_status(
                        "Host switching locked by DOER_REMOTE_URL (pass --allow-switch to enable)",
                        true,
                    );
                }
            }
            KeyCode::Char('/') => {
                self.modal = Some(Modal::Search {
                    input: self.search_query.clone().unwrap_or_default(),
                });
            }
            KeyCode::Char('r') => {
                self.reload_all();
                self.set_status("Refreshed all data", false);
            }

            // Pane Navigation
            KeyCode::Char('1') => self.focused_pane = Pane::Sidebar,
            KeyCode::Char('2') => self.focused_pane = Pane::TodoLists,
            KeyCode::Char('3') => self.focused_pane = Pane::Detail,
            KeyCode::Tab => self.focused_pane = self.focused_pane.next(),
            KeyCode::BackTab => self.focused_pane = self.focused_pane.prev(),
            KeyCode::Char('h') | KeyCode::Left => self.focused_pane = self.focused_pane.prev(),
            KeyCode::Char('l') | KeyCode::Right => self.focused_pane = self.focused_pane.next(),

            // Actions per pane
            _ => match self.focused_pane {
                Pane::Sidebar => self.handle_sidebar_key(key),
                Pane::TodoLists => self.handle_todo_lists_key(key),
                Pane::Detail => self.handle_detail_key(key),
            },
        }
    }

    fn handle_sidebar_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                if self.sidebar_selected + 1 < self.total_sidebar_items() {
                    self.sidebar_selected += 1;
                    self.todo_lists_selected = 0;
                    self.load_todo_lists();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.sidebar_selected > 0 {
                    self.sidebar_selected -= 1;
                    self.todo_lists_selected = 0;
                    self.load_todo_lists();
                }
            }
            KeyCode::Enter => {
                self.focused_pane = Pane::TodoLists;
            }
            KeyCode::Char('n') => {
                // New Project
                self.modal = Some(Modal::ProjectForm {
                    id: None,
                    name_input: String::new(),
                    desc_input: String::new(),
                    focus_desc: false,
                });
            }
            KeyCode::Char('e') => {
                // Edit selected project (if project selected)
                if let ScopeItem::Project(p) = self.get_sidebar_item(self.sidebar_selected) {
                    self.modal = Some(Modal::ProjectForm {
                        id: Some(p.id),
                        name_input: p.name,
                        desc_input: p.description.unwrap_or_default(),
                        focus_desc: false,
                    });
                }
            }
            KeyCode::Char('d') => {
                // Delete selected project (if project selected)
                if let ScopeItem::Project(p) = self.get_sidebar_item(self.sidebar_selected) {
                    self.modal = Some(Modal::ConfirmDelete {
                        target: DeleteTarget::Project(p),
                    });
                }
            }
            _ => {}
        }
    }

    fn handle_todo_lists_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.todo_lists.is_empty() && self.todo_lists_selected + 1 < self.todo_lists.len() {
                    self.todo_lists_selected += 1;
                    self.load_current_todo_list_detail();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.todo_lists_selected > 0 {
                    self.todo_lists_selected -= 1;
                    self.load_current_todo_list_detail();
                }
            }
            KeyCode::Enter => {
                self.focused_pane = Pane::Detail;
            }
            KeyCode::Char('p') => {
                self.toggle_pin_current();
            }
            KeyCode::Char('K') => {
                self.move_pinned_current(true);
            }
            KeyCode::Char('J') => {
                self.move_pinned_current(false);
            }
            KeyCode::Char('n') => {
                // New Todo List
                let default_proj_idx = match self.get_sidebar_item(self.sidebar_selected) {
                    ScopeItem::Project(p) => {
                        self.projects.iter().position(|pr| pr.id == p.id).map(|i| i + 1).unwrap_or(0)
                    }
                    _ => 0,
                };
                self.modal = Some(Modal::TodoListForm {
                    id: None,
                    title_input: String::new(),
                    project_choice_idx: default_proj_idx,
                    pinned: false,
                    focus_field: 0,
                });
            }
            KeyCode::Char('e') => {
                // Edit Todo List metadata
                if let Some(detail) = &self.current_todo_list {
                    let proj_choice = detail
                        .project_id
                        .and_then(|pid| self.projects.iter().position(|p| p.id == pid).map(|i| i + 1))
                        .unwrap_or(0);
                    self.modal = Some(Modal::TodoListForm {
                        id: Some(detail.id),
                        title_input: detail.title.clone(),
                        project_choice_idx: proj_choice,
                        pinned: detail.pinned,
                        focus_field: 0,
                    });
                }
            }
            KeyCode::Char('d') => {
                if let Some(tl) = self.todo_lists.get(self.todo_lists_selected) {
                    self.modal = Some(Modal::ConfirmDelete {
                        target: DeleteTarget::TodoList(tl.clone()),
                    });
                }
            }
            _ => {}
        }
    }

    fn handle_detail_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.checklist_items.is_empty() {
                    let next = match self.detail_selected_checklist {
                        Some(i) if i + 1 < self.checklist_items.len() => i + 1,
                        None => 0,
                        Some(i) => i,
                    };
                    self.detail_selected_checklist = Some(next);
                    self.adjust_scroll_to_checklist_item(next);
                } else {
                    self.detail_scroll = self.detail_scroll.saturating_add(1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.checklist_items.is_empty() {
                    let prev = match self.detail_selected_checklist {
                        Some(i) if i > 0 => i - 1,
                        _ => 0,
                    };
                    self.detail_selected_checklist = Some(prev);
                    self.adjust_scroll_to_checklist_item(prev);
                } else {
                    self.detail_scroll = self.detail_scroll.saturating_sub(1);
                }
            }
            // Document scrolling
            KeyCode::Char('J') => {
                self.detail_scroll = self.detail_scroll.saturating_add(3);
            }
            KeyCode::Char('K') => {
                self.detail_scroll = self.detail_scroll.saturating_sub(3);
            }
            KeyCode::PageDown => {
                self.detail_scroll = self.detail_scroll.saturating_add(10);
            }
            KeyCode::PageUp => {
                self.detail_scroll = self.detail_scroll.saturating_sub(10);
            }
            KeyCode::Char('g') => {
                self.detail_scroll = 0;
                if !self.checklist_items.is_empty() {
                    self.detail_selected_checklist = Some(0);
                }
            }
            KeyCode::Char('G') => {
                self.detail_scroll = 100;
                if !self.checklist_items.is_empty() {
                    self.detail_selected_checklist = Some(self.checklist_items.len() - 1);
                }
            }
            // Checklist toggle
            KeyCode::Char(' ') | KeyCode::Enter => {
                self.toggle_current_checklist_item();
            }
            // Quick add checklist item
            KeyCode::Char('a') => {
                self.modal = Some(Modal::QuickAddTask {
                    task_input: String::new(),
                    section_input: String::new(),
                    focus_section: false,
                });
            }
            // External editor
            KeyCode::Char('E') => {
                if let Err(e) = self.prepare_external_editor() {
                    self.set_status(format!("Failed to open editor: {}", e), true);
                }
            }
            // Yank
            KeyCode::Char('y') => {
                self.yank_current(false);
            }
            KeyCode::Char('Y') => {
                self.yank_current(true);
            }
            // Pin toggle
            KeyCode::Char('p') => {
                self.toggle_pin_current();
            }
            // Attachments
            KeyCode::Char('A') => {
                self.modal = Some(Modal::Attachments {
                    cursor: 0,
                    upload_input: None,
                    dest_input: None,
                });
            }
            // Edit TodoList metadata
            KeyCode::Char('e') => {
                if let Some(detail) = &self.current_todo_list {
                    let proj_choice = detail
                        .project_id
                        .and_then(|pid| self.projects.iter().position(|p| p.id == pid).map(|i| i + 1))
                        .unwrap_or(0);
                    self.modal = Some(Modal::TodoListForm {
                        id: Some(detail.id),
                        title_input: detail.title.clone(),
                        project_choice_idx: proj_choice,
                        pinned: detail.pinned,
                        focus_field: 0,
                    });
                }
            }
            _ => {}
        }
    }

    fn adjust_scroll_to_checklist_item(&mut self, item_idx: usize) {
        if let Some(item) = self.checklist_items.get(item_idx) {
            let line = item.line_number.saturating_sub(1);
            if line < self.detail_scroll {
                self.detail_scroll = line;
            } else if line > self.detail_scroll + 15 {
                self.detail_scroll = line.saturating_sub(10);
            }
        }
    }

    // Modal Event Handling
    fn handle_modal_key(&mut self, key: KeyCode) {
        let modal = self.modal.clone();
        match modal {
            Some(Modal::HostSwitcher {
                cursor,
                mut custom_input,
                entering_custom,
            }) => match key {
                KeyCode::Esc => self.modal = None,
                KeyCode::Up => {
                    if !entering_custom && cursor > 0 {
                        self.modal = Some(Modal::HostSwitcher {
                            cursor: cursor - 1,
                            custom_input,
                            entering_custom: false,
                        });
                    }
                }
                KeyCode::Down => {
                    if !entering_custom && cursor < self.preset_hosts.len() {
                        self.modal = Some(Modal::HostSwitcher {
                            cursor: cursor + 1,
                            custom_input,
                            entering_custom: false,
                        });
                    }
                }
                KeyCode::Tab => {
                    let next_entering = !entering_custom;
                    self.modal = Some(Modal::HostSwitcher {
                        cursor,
                        custom_input,
                        entering_custom: next_entering,
                    });
                }
                KeyCode::Char(c) if entering_custom => {
                    custom_input.push(c);
                    self.modal = Some(Modal::HostSwitcher {
                        cursor,
                        custom_input,
                        entering_custom: true,
                    });
                }
                KeyCode::Backspace if entering_custom => {
                    custom_input.pop();
                    self.modal = Some(Modal::HostSwitcher {
                        cursor,
                        custom_input,
                        entering_custom: true,
                    });
                }
                KeyCode::Enter => {
                    self.modal = None;
                    if entering_custom || cursor == self.preset_hosts.len() {
                        if !custom_input.trim().is_empty() {
                            self.switch_host(&custom_input);
                        }
                    } else if let Some(preset) = self.preset_hosts.get(cursor).cloned() {
                        self.switch_host(&preset);
                    }
                }
                _ => {}
            },

            Some(Modal::Search { mut input }) => match key {
                KeyCode::Esc => {
                    self.modal = None;
                    self.search_query = None;
                    self.load_todo_lists();
                }
                KeyCode::Enter => {
                    self.modal = None;
                    self.search_query = if input.trim().is_empty() {
                        None
                    } else {
                        Some(input.trim().to_string())
                    };
                    self.load_todo_lists();
                }
                KeyCode::Char(c) => {
                    input.push(c);
                    self.modal = Some(Modal::Search { input: input.clone() });
                    // Live search preview
                    self.search_query = Some(input);
                    self.load_todo_lists();
                }
                KeyCode::Backspace => {
                    input.pop();
                    self.modal = Some(Modal::Search { input: input.clone() });
                    self.search_query = if input.is_empty() { None } else { Some(input) };
                    self.load_todo_lists();
                }
                _ => {}
            },

            Some(Modal::QuickAddTask {
                mut task_input,
                mut section_input,
                focus_section,
            }) => match key {
                KeyCode::Esc => self.modal = None,
                KeyCode::Tab => {
                    self.modal = Some(Modal::QuickAddTask {
                        task_input,
                        section_input,
                        focus_section: !focus_section,
                    });
                }
                KeyCode::Enter => {
                    if !task_input.trim().is_empty() {
                        let sec = if section_input.trim().is_empty() {
                            None
                        } else {
                            Some(section_input.trim())
                        };
                        self.append_checklist_item(task_input.trim(), sec);
                    }
                    self.modal = None;
                }
                KeyCode::Char(c) => {
                    if focus_section {
                        section_input.push(c);
                    } else {
                        task_input.push(c);
                    }
                    self.modal = Some(Modal::QuickAddTask {
                        task_input,
                        section_input,
                        focus_section,
                    });
                }
                KeyCode::Backspace => {
                    if focus_section {
                        section_input.pop();
                    } else {
                        task_input.pop();
                    }
                    self.modal = Some(Modal::QuickAddTask {
                        task_input,
                        section_input,
                        focus_section,
                    });
                }
                _ => {}
            },

            Some(Modal::ProjectForm {
                id,
                mut name_input,
                mut desc_input,
                focus_desc,
            }) => match key {
                KeyCode::Esc => self.modal = None,
                KeyCode::Tab => {
                    self.modal = Some(Modal::ProjectForm {
                        id,
                        name_input,
                        desc_input,
                        focus_desc: !focus_desc,
                    });
                }
                KeyCode::Enter => {
                    if !name_input.trim().is_empty() {
                        let input = ProjectInput {
                            name: name_input.trim().to_string(),
                            description: if desc_input.trim().is_empty() {
                                None
                            } else {
                                Some(desc_input.trim().to_string())
                            },
                        };

                        let res = match id {
                            Some(pid) => self.client.update_project(pid, &input),
                            None => self.client.create_project(&input),
                        };

                        match res {
                            Ok(p) => {
                                self.set_status(format!("Saved project '{}'", p.name), false);
                                self.load_stats();
                                self.load_projects();
                            }
                            Err(e) => self.set_status(format!("Project save error: {}", e), true),
                        }
                    }
                    self.modal = None;
                }
                KeyCode::Char(c) => {
                    if focus_desc {
                        desc_input.push(c);
                    } else {
                        name_input.push(c);
                    }
                    self.modal = Some(Modal::ProjectForm {
                        id,
                        name_input,
                        desc_input,
                        focus_desc,
                    });
                }
                KeyCode::Backspace => {
                    if focus_desc {
                        desc_input.pop();
                    } else {
                        name_input.pop();
                    }
                    self.modal = Some(Modal::ProjectForm {
                        id,
                        name_input,
                        desc_input,
                        focus_desc,
                    });
                }
                _ => {}
            },

            Some(Modal::TodoListForm {
                id,
                mut title_input,
                project_choice_idx,
                mut pinned,
                focus_field,
            }) => match key {
                KeyCode::Esc => self.modal = None,
                KeyCode::Tab => {
                    let next = (focus_field + 1) % 3;
                    self.modal = Some(Modal::TodoListForm {
                        id,
                        title_input,
                        project_choice_idx,
                        pinned,
                        focus_field: next,
                    });
                }
                KeyCode::Left if focus_field == 1 => {
                    let next_idx = if project_choice_idx > 0 {
                        project_choice_idx - 1
                    } else {
                        self.projects.len()
                    };
                    self.modal = Some(Modal::TodoListForm {
                        id,
                        title_input,
                        project_choice_idx: next_idx,
                        pinned,
                        focus_field,
                    });
                }
                KeyCode::Right if focus_field == 1 => {
                    let next_idx = (project_choice_idx + 1) % (self.projects.len() + 1);
                    self.modal = Some(Modal::TodoListForm {
                        id,
                        title_input,
                        project_choice_idx: next_idx,
                        pinned,
                        focus_field,
                    });
                }
                KeyCode::Char(' ') if focus_field == 2 => {
                    pinned = !pinned;
                    self.modal = Some(Modal::TodoListForm {
                        id,
                        title_input,
                        project_choice_idx,
                        pinned,
                        focus_field,
                    });
                }
                KeyCode::Enter => {
                    if !title_input.trim().is_empty() {
                        let selected_pid = if project_choice_idx == 0 {
                            None
                        } else {
                            self.projects.get(project_choice_idx - 1).map(|p| p.id)
                        };

                        let input = TodoListInput {
                            title: title_input.trim().to_string(),
                            project_id: Some(selected_pid),
                            pinned: Some(pinned),
                            ..Default::default()
                        };

                        let res = match id {
                            Some(tlid) => self.client.update_todo_list(tlid, &input),
                            None => self.client.create_todo_list(&input),
                        };

                        match res {
                            Ok(tl) => {
                                self.set_status(format!("Saved todo list '{}'", tl.title), false);
                                self.load_stats();
                                self.load_todo_lists();
                            }
                            Err(e) => {
                                self.set_status(format!("Todo list save error: {}", e), true)
                            }
                        }
                    }
                    self.modal = None;
                }
                KeyCode::Char(c) if focus_field == 0 => {
                    title_input.push(c);
                    self.modal = Some(Modal::TodoListForm {
                        id,
                        title_input,
                        project_choice_idx,
                        pinned,
                        focus_field,
                    });
                }
                KeyCode::Backspace if focus_field == 0 => {
                    title_input.pop();
                    self.modal = Some(Modal::TodoListForm {
                        id,
                        title_input,
                        project_choice_idx,
                        pinned,
                        focus_field,
                    });
                }
                _ => {}
            },

            Some(Modal::ConfirmDelete { target }) => match key {
                KeyCode::Esc | KeyCode::Char('n') => self.modal = None,
                KeyCode::Char('y') | KeyCode::Enter => {
                    self.modal = None;
                    match target {
                        DeleteTarget::Project(p) => match self.client.delete_project(p.id) {
                            Ok(_) => {
                                self.set_status(format!("Deleted project '{}'", p.name), false);
                                self.sidebar_selected = 0;
                                self.load_stats();
                                self.load_projects();
                                self.load_todo_lists();
                            }
                            Err(e) => {
                                self.set_status(format!("Delete project error: {}", e), true)
                            }
                        },
                        DeleteTarget::TodoList(tl) => match self.client.delete_todo_list(tl.id) {
                            Ok(_) => {
                                self.set_status(format!("Deleted todo list '{}'", tl.title), false);
                                self.load_stats();
                                self.load_todo_lists();
                            }
                            Err(e) => {
                                self.set_status(format!("Delete todo list error: {}", e), true)
                            }
                        },
                        DeleteTarget::Attachment {
                            todo_list_id,
                            attachment_id,
                            filename,
                        } => {
                            match self.client.delete_attachment(todo_list_id, attachment_id) {
                                Ok(_) => {
                                    self.set_status(format!("Deleted attachment {}", filename), false);
                                    self.load_stats();
                                    self.load_current_todo_list_detail();
                                }
                                Err(e) => {
                                    self.set_status(format!("Delete attachment error: {}", e), true)
                                }
                            }
                        }
                    }
                }
                _ => {}
            },

            Some(Modal::Attachments {
                cursor,
                upload_input,
                dest_input,
            }) => {
                let attachments = self
                    .current_todo_list
                    .as_ref()
                    .map(|d| d.attachments.clone())
                    .unwrap_or_default();

                if let Some(mut path_str) = upload_input {
                    match key {
                        KeyCode::Esc => {
                            self.modal = Some(Modal::Attachments {
                                cursor,
                                upload_input: None,
                                dest_input: None,
                            });
                        }
                        KeyCode::Enter => {
                            let p = PathBuf::from(path_str.trim());
                            if p.exists() {
                                if let Some(d) = &self.current_todo_list {
                                    match self.client.upload_attachment(d.id, &p) {
                                        Ok(_) => {
                                            self.set_status("Attachment uploaded", false);
                                            self.load_current_todo_list_detail();
                                        }
                                        Err(e) => self
                                            .set_status(format!("Upload error: {}", e), true),
                                    }
                                }
                            } else {
                                self.set_status("File path does not exist", true);
                            }
                            self.modal = Some(Modal::Attachments {
                                cursor,
                                upload_input: None,
                                dest_input: None,
                            });
                        }
                        KeyCode::Char(c) => {
                            path_str.push(c);
                            self.modal = Some(Modal::Attachments {
                                cursor,
                                upload_input: Some(path_str),
                                dest_input: None,
                            });
                        }
                        KeyCode::Backspace => {
                            path_str.pop();
                            self.modal = Some(Modal::Attachments {
                                cursor,
                                upload_input: Some(path_str),
                                dest_input: None,
                            });
                        }
                        _ => {}
                    }
                    return;
                }

                if let Some(mut save_str) = dest_input {
                    match key {
                        KeyCode::Esc => {
                            self.modal = Some(Modal::Attachments {
                                cursor,
                                upload_input: None,
                                dest_input: None,
                            });
                        }
                        KeyCode::Enter => {
                            if let Some(att) = attachments.get(cursor) {
                                let p = PathBuf::from(save_str.trim());
                                match self.client.download_attachment(&att.url, &p) {
                                    Ok(_) => self.set_status(
                                        format!("Downloaded attachment to {:?}", p),
                                        false,
                                    ),
                                    Err(e) => self
                                        .set_status(format!("Download error: {}", e), true),
                                }
                            }
                            self.modal = Some(Modal::Attachments {
                                cursor,
                                upload_input: None,
                                dest_input: None,
                            });
                        }
                        KeyCode::Char(c) => {
                            save_str.push(c);
                            self.modal = Some(Modal::Attachments {
                                cursor,
                                upload_input: None,
                                dest_input: Some(save_str),
                            });
                        }
                        KeyCode::Backspace => {
                            save_str.pop();
                            self.modal = Some(Modal::Attachments {
                                cursor,
                                upload_input: None,
                                dest_input: Some(save_str),
                            });
                        }
                        _ => {}
                    }
                    return;
                }

                match key {
                    KeyCode::Esc => self.modal = None,
                    KeyCode::Up | KeyCode::Char('k') => {
                        if cursor > 0 {
                            self.modal = Some(Modal::Attachments {
                                cursor: cursor - 1,
                                upload_input: None,
                                dest_input: None,
                            });
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !attachments.is_empty() && cursor + 1 < attachments.len() {
                            self.modal = Some(Modal::Attachments {
                                cursor: cursor + 1,
                                upload_input: None,
                                dest_input: None,
                            });
                        }
                    }
                    KeyCode::Char('u') => {
                        self.modal = Some(Modal::Attachments {
                            cursor,
                            upload_input: Some(String::new()),
                            dest_input: None,
                        });
                    }
                    KeyCode::Char('s') => {
                        if let Some(att) = attachments.get(cursor) {
                            let default_dest = format!("./{}", att.filename);
                            self.modal = Some(Modal::Attachments {
                                cursor,
                                upload_input: None,
                                dest_input: Some(default_dest),
                            });
                        }
                    }
                    KeyCode::Char('y') => {
                        if let Some(att) = attachments.get(cursor) {
                            if let Some(cb) = &mut self.clipboard {
                                let _ = cb.set_text(att.url.clone());
                                self.set_status("Copied attachment URL", false);
                            }
                        }
                    }
                    KeyCode::Char('d') => {
                        if let Some(att) = attachments.get(cursor) {
                            if let Some(d) = &self.current_todo_list {
                                self.modal = Some(Modal::ConfirmDelete {
                                    target: DeleteTarget::Attachment {
                                        todo_list_id: d.id,
                                        attachment_id: att.id,
                                        filename: att.filename.clone(),
                                    },
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }

            Some(Modal::Help) => match key {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('?') | KeyCode::Char('q') => {
                    self.modal = None;
                }
                _ => {}
            },

            None => {}
        }
    }
}
