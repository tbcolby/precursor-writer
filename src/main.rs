mod editor;
mod export;
mod journal;
mod render;
mod storage;
mod typewriter;
mod ui;

use num_traits::ToPrimitive;
use num_traits::FromPrimitive;

use crate::editor::EditorState;
use crate::journal::JournalState;
use crate::typewriter::TypewriterState;
use crate::storage::WriterStorage;
use crate::render::Renderer;
use crate::export::ExportSystem;

const SERVER_NAME: &str = "_Writer_";
const APP_NAME: &str = "Writer";

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AppMode {
    ModeSelect,
    DocList,
    EditorEdit,
    EditorPreview,
    FileMenu,
    ExportMenu,
    JournalDay,
    JournalNav,
    JournalSearch,
    TypewriterEdit,
    TypewriterDone,
}

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
enum AppOp {
    Redraw = 0,
    Rawkeys,
    FocusChange,
    Quit,
}

pub struct WriterApp {
    mode: AppMode,
    mode_cursor: usize,
    allow_redraw: bool,
    renderer: Renderer,
    storage: WriterStorage,
    export: ExportSystem,
    editor: EditorState,
    journal: JournalState,
    typewriter: TypewriterState,
    esc_pending: bool,
    // Doc list state
    doc_list: Vec<String>,
    doc_cursor: usize,
    // File menu state
    file_menu_cursor: usize,
    // Export menu state
    export_menu_cursor: usize,
}

impl WriterApp {
    pub fn new(xns: &xous_names::XousNames, sid: xous::SID) -> Self {
        let gam = gam::Gam::new(xns).expect("can't connect to GAM");

        let token = gam
            .register_ux(gam::UxRegistration {
                app_name: String::from(APP_NAME),
                ux_type: gam::UxType::Chat,
                predictor: None,
                listener: sid.to_array(),
                redraw_id: AppOp::Redraw.to_u32().unwrap(),
                gotinput_id: None,
                audioframe_id: None,
                rawkeys_id: Some(AppOp::Rawkeys.to_u32().unwrap()),
                focuschange_id: Some(AppOp::FocusChange.to_u32().unwrap()),
            })
            .expect("couldn't register UX")
            .unwrap();

        let content = gam.request_content_canvas(token).expect("couldn't get canvas");
        let screensize = gam.get_canvas_bounds(content).expect("couldn't get dimensions");

        let renderer = Renderer::new(gam, content, screensize);
        let storage = WriterStorage::new();
        let export = ExportSystem::new();

        Self {
            mode: AppMode::ModeSelect,
            mode_cursor: 0,
            allow_redraw: true,
            renderer,
            storage,
            export,
            editor: EditorState::new(),
            journal: JournalState::new(),
            typewriter: TypewriterState::new(),
            esc_pending: false,
            doc_list: Vec::new(),
            doc_cursor: 0,
            file_menu_cursor: 0,
            export_menu_cursor: 0,
        }
    }

    pub fn redraw(&mut self) {
        if !self.allow_redraw {
            return;
        }
        match self.mode {
            AppMode::ModeSelect => self.renderer.draw_mode_select(self.mode_cursor),
            AppMode::DocList => self.renderer.draw_doc_list(&self.doc_list, self.doc_cursor),
            AppMode::EditorEdit => {
                self.renderer.draw_editor(&self.editor.buffer, &self.editor.doc_name, false);
            }
            AppMode::EditorPreview => {
                self.renderer.draw_editor(&self.editor.buffer, &self.editor.doc_name, true);
            }
            AppMode::FileMenu => {
                self.renderer.draw_file_menu(self.file_menu_cursor);
            }
            AppMode::ExportMenu => {
                self.renderer.draw_export_menu(self.export_menu_cursor);
            }
            AppMode::JournalDay => {
                self.renderer.draw_journal(&self.journal.buffer, &self.journal.current_date);
            }
            AppMode::JournalSearch => {
                self.renderer.draw_journal_search(&self.journal.search_query, &self.journal.search_results);
            }
            AppMode::TypewriterEdit => {
                self.renderer.draw_typewriter(&self.typewriter.buffer);
            }
            AppMode::TypewriterDone => {
                self.renderer.draw_typewriter_done(
                    self.typewriter.buffer.word_count(),
                    self.typewriter.buffer.char_count(),
                    self.typewriter.buffer.line_count(),
                );
            }
            _ => {}
        }
    }

    pub fn handle_key(&mut self, key: char) {
        // Handle escape sequences
        if self.esc_pending {
            self.esc_pending = false;
            self.handle_esc_command(key);
            return;
        }

        if key == '\u{001b}' {
            // ESC character
            self.esc_pending = true;
            return;
        }

        match self.mode {
            AppMode::ModeSelect => self.handle_key_mode_select(key),
            AppMode::DocList => self.handle_key_doc_list(key),
            AppMode::EditorEdit => self.handle_key_editor(key),
            AppMode::EditorPreview => self.handle_key_preview(key),
            AppMode::FileMenu => self.handle_key_file_menu(key),
            AppMode::ExportMenu => self.handle_key_export_menu(key),
            AppMode::JournalDay => self.handle_key_journal(key),
            AppMode::JournalSearch => self.handle_key_journal_search(key),
            AppMode::TypewriterEdit => self.handle_key_typewriter(key),
            AppMode::TypewriterDone => self.handle_key_typewriter_done(key),
            _ => {}
        }
    }

    fn handle_esc_command(&mut self, key: char) {
        match self.mode {
            AppMode::EditorEdit => {
                match key {
                    'p' => {
                        self.mode = AppMode::EditorPreview;
                        self.redraw();
                    }
                    's' => {
                        self.save_current_doc();
                    }
                    'e' => {
                        self.export_menu_cursor = 0;
                        self.mode = AppMode::ExportMenu;
                        self.redraw();
                    }
                    'f' => {
                        self.file_menu_cursor = 0;
                        self.mode = AppMode::FileMenu;
                        self.redraw();
                    }
                    'q' => {
                        self.save_current_doc();
                        self.refresh_doc_list();
                        self.mode = AppMode::DocList;
                        self.redraw();
                    }
                    _ => {}
                }
            }
            AppMode::EditorPreview => {
                match key {
                    'p' => {
                        self.mode = AppMode::EditorEdit;
                        self.redraw();
                    }
                    'q' => {
                        self.save_current_doc();
                        self.refresh_doc_list();
                        self.mode = AppMode::DocList;
                        self.redraw();
                    }
                    _ => {}
                }
            }
            AppMode::JournalDay => {
                match key {
                    '[' => {
                        self.journal.save_entry(&self.storage);
                        self.journal.prev_day(&self.storage);
                        self.redraw();
                    }
                    ']' => {
                        self.journal.save_entry(&self.storage);
                        self.journal.next_day(&self.storage);
                        self.redraw();
                    }
                    't' => {
                        self.journal.save_entry(&self.storage);
                        self.journal.jump_to_today();
                        self.journal.load_entry(&self.storage);
                        self.redraw();
                    }
                    '/' => {
                        self.journal.search_query.clear();
                        self.journal.search_results.clear();
                        self.mode = AppMode::JournalSearch;
                        self.redraw();
                    }
                    's' => {
                        self.journal.save_entry(&self.storage);
                        self.redraw();
                    }
                    'q' => {
                        self.journal.save_entry(&self.storage);
                        self.mode = AppMode::ModeSelect;
                        self.redraw();
                    }
                    _ => {}
                }
            }
            AppMode::TypewriterEdit => {
                match key {
                    'd' => {
                        self.mode = AppMode::TypewriterDone;
                        self.redraw();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn handle_key_mode_select(&mut self, key: char) {
        match key {
            '\u{F700}' | '↑' => {
                if self.mode_cursor > 0 {
                    self.mode_cursor -= 1;
                    self.redraw();
                }
            }
            '\u{F701}' | '↓' => {
                if self.mode_cursor < 2 {
                    self.mode_cursor += 1;
                    self.redraw();
                }
            }
            '\r' | '\n' => {
                match self.mode_cursor {
                    0 => {
                        self.refresh_doc_list();
                        self.mode = AppMode::DocList;
                    }
                    1 => {
                        self.journal.jump_to_today();
                        self.journal.load_entry(&self.storage);
                        self.mode = AppMode::JournalDay;
                    }
                    2 => {
                        self.typewriter = TypewriterState::new();
                        self.mode = AppMode::TypewriterEdit;
                    }
                    _ => {}
                }
                self.redraw();
            }
            'q' => {
                // Quit app - could send quit message
            }
            _ => {}
        }
    }

    fn handle_key_doc_list(&mut self, key: char) {
        match key {
            '\u{F700}' | '↑' => {
                if self.doc_cursor > 0 {
                    self.doc_cursor -= 1;
                    self.redraw();
                }
            }
            '\u{F701}' | '↓' => {
                if self.doc_cursor + 1 < self.doc_list.len() {
                    self.doc_cursor += 1;
                    self.redraw();
                }
            }
            '\r' | '\n' => {
                if !self.doc_list.is_empty() {
                    let name = self.doc_list[self.doc_cursor].clone();
                    self.open_doc(&name);
                }
            }
            'n' => {
                self.new_doc();
            }
            'd' => {
                if !self.doc_list.is_empty() {
                    let name = self.doc_list[self.doc_cursor].clone();
                    self.storage.delete_doc(&name);
                    self.refresh_doc_list();
                    if self.doc_cursor >= self.doc_list.len() && self.doc_cursor > 0 {
                        self.doc_cursor -= 1;
                    }
                    self.redraw();
                }
            }
            'q' => {
                self.mode = AppMode::ModeSelect;
                self.redraw();
            }
            _ => {}
        }
    }

    fn handle_key_editor(&mut self, key: char) {
        match key {
            '\u{F700}' | '↑' => {
                self.editor.buffer.move_up();
                self.redraw();
            }
            '\u{F701}' | '↓' => {
                self.editor.buffer.move_down();
                self.redraw();
            }
            '\u{F702}' | '←' => {
                self.editor.buffer.move_left();
                self.redraw();
            }
            '\u{F703}' | '→' => {
                self.editor.buffer.move_right();
                self.redraw();
            }
            '\r' | '\n' => {
                self.editor.buffer.newline();
                self.redraw();
            }
            '\u{0008}' | '\u{007f}' => {
                // Backspace
                self.editor.buffer.delete_back();
                self.redraw();
            }
            '\u{F728}' => {
                // Delete key
                self.editor.buffer.delete_forward();
                self.redraw();
            }
            '\u{F729}' => {
                // Home key
                self.editor.buffer.move_home();
                self.redraw();
            }
            '\u{F72B}' => {
                // End key
                self.editor.buffer.move_end();
                self.redraw();
            }
            ch if !ch.is_control() => {
                self.editor.buffer.insert_char(ch);
                self.redraw();
            }
            _ => {}
        }
    }

    fn handle_key_preview(&mut self, _key: char) {
        // In preview mode, most keys are ignored
        // Esc commands handled in handle_esc_command
    }

    fn handle_key_file_menu(&mut self, key: char) {
        match key {
            '\u{F700}' | '↑' => {
                if self.file_menu_cursor > 0 {
                    self.file_menu_cursor -= 1;
                    self.redraw();
                }
            }
            '\u{F701}' | '↓' => {
                if self.file_menu_cursor < 2 {
                    self.file_menu_cursor += 1;
                    self.redraw();
                }
            }
            '\r' | '\n' => {
                match self.file_menu_cursor {
                    0 => {
                        // New document
                        self.save_current_doc();
                        self.new_doc();
                    }
                    1 => {
                        // Delete current
                        let name = self.editor.doc_name.clone();
                        if !name.is_empty() {
                            self.storage.delete_doc(&name);
                        }
                        self.refresh_doc_list();
                        self.mode = AppMode::DocList;
                        self.redraw();
                    }
                    2 => {
                        // Back to editor
                        self.mode = AppMode::EditorEdit;
                        self.redraw();
                    }
                    _ => {}
                }
            }
            'q' => {
                self.mode = AppMode::EditorEdit;
                self.redraw();
            }
            _ => {}
        }
    }

    fn handle_key_export_menu(&mut self, key: char) {
        match key {
            '\u{F700}' | '↑' => {
                if self.export_menu_cursor > 0 {
                    self.export_menu_cursor -= 1;
                    self.redraw();
                }
            }
            '\u{F701}' | '↓' => {
                if self.export_menu_cursor < 1 {
                    self.export_menu_cursor += 1;
                    self.redraw();
                }
            }
            '\r' | '\n' => {
                let content = self.editor.buffer.to_string();
                match self.export_menu_cursor {
                    0 => {
                        // TCP export
                        self.export.export_tcp(&content);
                    }
                    1 => {
                        // USB autotype
                        self.export.export_usb_autotype(&content);
                    }
                    _ => {}
                }
                self.mode = AppMode::EditorEdit;
                self.redraw();
            }
            'q' => {
                self.mode = AppMode::EditorEdit;
                self.redraw();
            }
            _ => {}
        }
    }

    fn handle_key_journal(&mut self, key: char) {
        match key {
            '\u{F700}' | '↑' => {
                self.journal.buffer.move_up();
                self.redraw();
            }
            '\u{F701}' | '↓' => {
                self.journal.buffer.move_down();
                self.redraw();
            }
            '\u{F702}' | '←' => {
                self.journal.buffer.move_left();
                self.redraw();
            }
            '\u{F703}' | '→' => {
                self.journal.buffer.move_right();
                self.redraw();
            }
            '\r' | '\n' => {
                self.journal.buffer.newline();
                self.redraw();
            }
            '\u{0008}' | '\u{007f}' => {
                self.journal.buffer.delete_back();
                self.redraw();
            }
            ch if !ch.is_control() => {
                self.journal.buffer.insert_char(ch);
                self.redraw();
            }
            _ => {}
        }
    }

    fn handle_key_journal_search(&mut self, key: char) {
        match key {
            '\r' | '\n' => {
                // Execute search
                self.journal.search_entries(&self.storage);
                self.redraw();
            }
            '\u{0008}' | '\u{007f}' => {
                self.journal.search_query.pop();
                self.redraw();
            }
            'q' if self.journal.search_query.is_empty() => {
                self.mode = AppMode::JournalDay;
                self.redraw();
            }
            ch if !ch.is_control() => {
                self.journal.search_query.push(ch);
                self.redraw();
            }
            _ => {
                // Esc handled by esc_pending, 'q' when empty exits
                self.mode = AppMode::JournalDay;
                self.redraw();
            }
        }
    }

    fn handle_key_typewriter(&mut self, key: char) {
        match key {
            '\r' | '\n' => {
                self.typewriter.buffer.append_newline();
                self.redraw();
            }
            ch if !ch.is_control() => {
                self.typewriter.buffer.append_char(ch);
                self.redraw();
            }
            _ => {
                // No backspace, no cursor movement in typewriter mode
            }
        }
    }

    fn handle_key_typewriter_done(&mut self, key: char) {
        match key {
            's' => {
                // Save as document
                let content = self.typewriter.buffer.to_string();
                let name = self.storage.next_doc_name("Freewrite");
                self.storage.save_doc(&name, &content);
                self.mode = AppMode::ModeSelect;
                self.redraw();
            }
            'q' => {
                // Discard
                self.mode = AppMode::ModeSelect;
                self.redraw();
            }
            _ => {}
        }
    }

    // Document management helpers

    fn refresh_doc_list(&mut self) {
        self.doc_list = self.storage.list_docs();
        if self.doc_cursor >= self.doc_list.len() {
            self.doc_cursor = self.doc_list.len().saturating_sub(1);
        }
    }

    fn new_doc(&mut self) {
        let name = self.storage.next_doc_name("Untitled");
        self.editor = EditorState::with_name(&name);
        self.mode = AppMode::EditorEdit;
        self.redraw();
    }

    fn open_doc(&mut self, name: &str) {
        if let Some(content) = self.storage.load_doc(name) {
            self.editor = EditorState::with_content(name, &content);
        } else {
            self.editor = EditorState::with_name(name);
        }
        self.mode = AppMode::EditorEdit;
        self.redraw();
    }

    fn save_current_doc(&mut self) {
        if !self.editor.doc_name.is_empty() {
            let content = self.editor.buffer.to_string();
            self.storage.save_doc(&self.editor.doc_name, &content);
            self.editor.buffer.modified = false;
        }
    }
}

fn main() -> ! {
    log_server::init_wait().unwrap();
    log::set_max_level(log::LevelFilter::Info);
    log::info!("Writer PID is {}", xous::process::id());

    let xns = xous_names::XousNames::new().unwrap();
    let sid = xns.register_name(SERVER_NAME, None).expect("can't register server");

    let mut app = WriterApp::new(&xns, sid);
    app.allow_redraw = true;

    loop {
        let msg = xous::receive_message(sid).unwrap();
        match FromPrimitive::from_usize(msg.body.id()) {
            Some(AppOp::Redraw) => {
                app.redraw();
            }
            Some(AppOp::Rawkeys) => xous::msg_scalar_unpack!(msg, k1, k2, k3, k4, {
                let keys = [
                    core::char::from_u32(k1 as u32).unwrap_or('\u{0000}'),
                    core::char::from_u32(k2 as u32).unwrap_or('\u{0000}'),
                    core::char::from_u32(k3 as u32).unwrap_or('\u{0000}'),
                    core::char::from_u32(k4 as u32).unwrap_or('\u{0000}'),
                ];
                for &key in keys.iter() {
                    if key != '\u{0000}' {
                        app.handle_key(key);
                    }
                }
            }),
            Some(AppOp::FocusChange) => xous::msg_scalar_unpack!(msg, new_state_code, _, _, _, {
                let new_state = gam::FocusState::convert_focus_change(new_state_code);
                match new_state {
                    gam::FocusState::Background => {
                        app.allow_redraw = false;
                        // Auto-save on background
                        app.save_current_doc();
                        if app.mode == AppMode::JournalDay {
                            app.journal.save_entry(&app.storage);
                        }
                    }
                    gam::FocusState::Foreground => {
                        app.allow_redraw = true;
                        app.redraw();
                    }
                }
            }),
            Some(AppOp::Quit) => break,
            _ => log::error!("unknown opcode: {:?}", msg),
        }
    }

    xns.unregister_server(sid).unwrap();
    xous::destroy_server(sid).unwrap();
    xous::terminate_process(0)
}
