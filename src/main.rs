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

// F-key character codes from Xous keyboard service
const KEY_F1: char = '\u{0011}';
const KEY_F2: char = '\u{0012}';
const KEY_F3: char = '\u{0013}';
const KEY_F4: char = '\u{0014}';

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
    HelpScreen,
    ConfirmExit,
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
    // F-key menu overlay state
    menu_visible: bool,
    menu_cursor: usize,
    // Mode before help/confirm (to return to)
    prev_mode: AppMode,
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
            menu_visible: false,
            menu_cursor: 0,
            prev_mode: AppMode::ModeSelect,
        }
    }

    pub fn redraw(&mut self) {
        if !self.allow_redraw {
            return;
        }

        if self.menu_visible {
            self.renderer.draw_menu(self.menu_items(), self.menu_cursor);
            return;
        }

        match self.mode {
            AppMode::HelpScreen => {
                self.renderer.draw_help(self.help_text());
            }
            AppMode::ConfirmExit => {
                self.renderer.draw_confirm_exit();
            }
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
        // F-keys always processed first (clear any pending ESC)
        match key {
            KEY_F1 => { self.esc_pending = false; self.toggle_menu(); return; }
            KEY_F4 => { self.esc_pending = false; self.handle_f4(); return; }
            KEY_F2 => { self.esc_pending = false; self.handle_f2(); return; }
            KEY_F3 => { self.esc_pending = false; self.handle_f3(); return; }
            _ => {}
        }

        // If menu is open, handle menu navigation only
        if self.menu_visible {
            match key {
                '\u{F700}' | '↑' => {
                    if self.menu_cursor > 0 {
                        self.menu_cursor -= 1;
                        self.redraw();
                    }
                }
                '\u{F701}' | '↓' => {
                    let items = self.menu_items();
                    if self.menu_cursor + 1 < items.len() {
                        self.menu_cursor += 1;
                        self.redraw();
                    }
                }
                '\r' | '\n' => {
                    self.menu_select_item();
                }
                _ => {}
            }
            return;
        }

        // Help screen - any key returns to previous mode
        if self.mode == AppMode::HelpScreen {
            self.mode = self.prev_mode;
            self.redraw();
            return;
        }

        // Confirm exit dialog
        if self.mode == AppMode::ConfirmExit {
            match key {
                'y' => {
                    self.save_current_doc();
                    self.refresh_doc_list();
                    self.mode = AppMode::DocList;
                    self.redraw();
                }
                'n' => {
                    self.editor.buffer.modified = false;
                    self.refresh_doc_list();
                    self.mode = AppMode::DocList;
                    self.redraw();
                }
                _ => {}
            }
            return;
        }

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

    fn menu_items(&self) -> &'static [&'static str] {
        match self.mode {
            AppMode::EditorEdit | AppMode::EditorPreview => {
                &["Help", "Save", "Export", "File Menu", "Toggle Preview"]
            }
            AppMode::JournalDay => {
                &["Help", "Prev Day", "Next Day", "Today", "Search"]
            }
            AppMode::TypewriterEdit => {
                &["Help", "Done (summary)"]
            }
            AppMode::DocList => &["Help", "New Document", "Back"],
            AppMode::ModeSelect => &["Help"],
            AppMode::TypewriterDone => &["Help", "Save as Doc", "Discard"],
            AppMode::FileMenu => &["Help", "Back to Editor"],
            AppMode::ExportMenu => &["Help", "Back to Editor"],
            AppMode::JournalSearch => &["Help", "Back to Journal"],
            _ => &["Help"],
        }
    }

    fn toggle_menu(&mut self) {
        if self.mode == AppMode::HelpScreen || self.mode == AppMode::ConfirmExit {
            return;
        }
        self.menu_visible = !self.menu_visible;
        self.menu_cursor = 0;
        self.redraw();
    }

    fn menu_select_item(&mut self) {
        self.menu_visible = false;

        match self.mode {
            AppMode::EditorEdit | AppMode::EditorPreview => {
                match self.menu_cursor {
                    0 => {
                        self.prev_mode = self.mode;
                        self.mode = AppMode::HelpScreen;
                    }
                    1 => { self.save_current_doc(); }
                    2 => {
                        self.export_menu_cursor = 0;
                        self.mode = AppMode::ExportMenu;
                    }
                    3 => {
                        self.file_menu_cursor = 0;
                        self.mode = AppMode::FileMenu;
                    }
                    4 => {
                        self.mode = if self.mode == AppMode::EditorEdit {
                            AppMode::EditorPreview
                        } else {
                            AppMode::EditorEdit
                        };
                    }
                    _ => {}
                }
            }
            AppMode::JournalDay => {
                match self.menu_cursor {
                    0 => {
                        self.prev_mode = self.mode;
                        self.mode = AppMode::HelpScreen;
                    }
                    1 => {
                        self.journal.save_entry(&self.storage);
                        self.journal.prev_day(&self.storage);
                    }
                    2 => {
                        self.journal.save_entry(&self.storage);
                        self.journal.next_day(&self.storage);
                    }
                    3 => {
                        self.journal.save_entry(&self.storage);
                        self.journal.jump_to_today();
                        self.journal.load_entry(&self.storage);
                    }
                    4 => {
                        self.journal.search_query.clear();
                        self.journal.search_results.clear();
                        self.mode = AppMode::JournalSearch;
                    }
                    _ => {}
                }
            }
            AppMode::TypewriterEdit => {
                match self.menu_cursor {
                    0 => {
                        self.prev_mode = self.mode;
                        self.mode = AppMode::HelpScreen;
                    }
                    1 => { self.mode = AppMode::TypewriterDone; }
                    _ => {}
                }
            }
            AppMode::DocList => {
                match self.menu_cursor {
                    0 => {
                        self.prev_mode = self.mode;
                        self.mode = AppMode::HelpScreen;
                    }
                    1 => { self.new_doc(); return; }
                    2 => { self.mode = AppMode::ModeSelect; }
                    _ => {}
                }
            }
            AppMode::TypewriterDone => {
                match self.menu_cursor {
                    0 => {
                        self.prev_mode = self.mode;
                        self.mode = AppMode::HelpScreen;
                    }
                    1 => {
                        let content = self.typewriter.buffer.to_string();
                        let name = self.storage.next_doc_name("Freewrite");
                        self.storage.save_doc(&name, &content);
                        self.mode = AppMode::ModeSelect;
                    }
                    2 => { self.mode = AppMode::ModeSelect; }
                    _ => {}
                }
            }
            AppMode::FileMenu => {
                match self.menu_cursor {
                    0 => {
                        self.prev_mode = self.mode;
                        self.mode = AppMode::HelpScreen;
                    }
                    1 => { self.mode = AppMode::EditorEdit; }
                    _ => {}
                }
            }
            AppMode::ExportMenu => {
                match self.menu_cursor {
                    0 => {
                        self.prev_mode = self.mode;
                        self.mode = AppMode::HelpScreen;
                    }
                    1 => { self.mode = AppMode::EditorEdit; }
                    _ => {}
                }
            }
            AppMode::JournalSearch => {
                match self.menu_cursor {
                    0 => {
                        self.prev_mode = self.mode;
                        self.mode = AppMode::HelpScreen;
                    }
                    1 => { self.mode = AppMode::JournalDay; }
                    _ => {}
                }
            }
            _ => {
                // Help is always item 0
                if self.menu_cursor == 0 {
                    self.prev_mode = self.mode;
                    self.mode = AppMode::HelpScreen;
                }
            }
        }
        self.redraw();
    }

    fn handle_f2(&mut self) {
        if self.menu_visible { self.menu_visible = false; }
        if self.mode == AppMode::HelpScreen || self.mode == AppMode::ConfirmExit { return; }
        // F2 = Toggle Preview (in editor modes)
        match self.mode {
            AppMode::EditorEdit => { self.mode = AppMode::EditorPreview; }
            AppMode::EditorPreview => { self.mode = AppMode::EditorEdit; }
            _ => {}
        }
        self.redraw();
    }

    fn handle_f3(&mut self) {
        if self.menu_visible { self.menu_visible = false; }
        if self.mode == AppMode::HelpScreen || self.mode == AppMode::ConfirmExit { return; }
        // F3 = Save
        match self.mode {
            AppMode::EditorEdit | AppMode::EditorPreview => {
                self.save_current_doc();
            }
            AppMode::JournalDay => {
                self.journal.save_entry(&self.storage);
            }
            _ => {}
        }
        self.redraw();
    }

    fn handle_f4(&mut self) {
        // F4 closes menu first
        if self.menu_visible {
            self.menu_visible = false;
            self.redraw();
            return;
        }
        // F4 closes help screen
        if self.mode == AppMode::HelpScreen {
            self.mode = self.prev_mode;
            self.redraw();
            return;
        }
        // F4 cancels confirm exit
        if self.mode == AppMode::ConfirmExit {
            self.mode = self.prev_mode;
            self.redraw();
            return;
        }
        // F4 = Back/Exit with unsaved changes confirmation
        match self.mode {
            AppMode::EditorEdit | AppMode::EditorPreview => {
                if self.editor.buffer.modified {
                    self.prev_mode = self.mode;
                    self.mode = AppMode::ConfirmExit;
                    self.redraw();
                } else {
                    self.refresh_doc_list();
                    self.mode = AppMode::DocList;
                    self.redraw();
                }
            }
            AppMode::DocList => {
                self.mode = AppMode::ModeSelect;
                self.redraw();
            }
            AppMode::FileMenu | AppMode::ExportMenu => {
                self.mode = AppMode::EditorEdit;
                self.redraw();
            }
            AppMode::JournalDay => {
                self.journal.save_entry(&self.storage);
                self.mode = AppMode::ModeSelect;
                self.redraw();
            }
            AppMode::JournalSearch => {
                self.mode = AppMode::JournalDay;
                self.redraw();
            }
            AppMode::TypewriterEdit => {
                self.mode = AppMode::TypewriterDone;
                self.redraw();
            }
            AppMode::TypewriterDone => {
                self.mode = AppMode::ModeSelect;
                self.redraw();
            }
            AppMode::ModeSelect => {
                // Top level - quit
            }
            _ => {}
        }
    }

    fn help_text(&self) -> &'static str {
        match self.prev_mode {
            AppMode::EditorEdit | AppMode::EditorPreview => {
                "EDITOR HELP\n\n\
                 F1     Menu\n\
                 F2     Toggle Preview\n\
                 F3     Save\n\
                 F4     Back to doc list\n\n\
                 Arrows Move cursor\n\
                 Esc+p  Toggle Preview\n\
                 Esc+s  Save\n\
                 Esc+e  Export menu\n\
                 Esc+f  File menu\n\
                 Esc+q  Back to doc list"
            }
            AppMode::DocList => {
                "DOCUMENTS HELP\n\n\
                 F1     Menu\n\
                 F4     Back\n\n\
                 Enter  Open document\n\
                 n      New document\n\
                 d      Delete document\n\
                 q      Back"
            }
            AppMode::JournalDay => {
                "JOURNAL HELP\n\n\
                 F1     Menu\n\
                 F3     Save\n\
                 F4     Back\n\n\
                 Esc+[  Previous day\n\
                 Esc+]  Next day\n\
                 Esc+t  Today\n\
                 Esc+/  Search\n\
                 Esc+s  Save\n\
                 Esc+q  Back"
            }
            AppMode::TypewriterEdit => {
                "TYPEWRITER HELP\n\n\
                 F1     Menu\n\
                 F4     Done (summary)\n\n\
                 Type freely!\n\
                 No backspace.\n\
                 No cursor movement.\n\n\
                 Esc+d  Done (summary)"
            }
            AppMode::ModeSelect => {
                "WRITER HELP\n\n\
                 F1     Menu\n\
                 F4     Quit\n\n\
                 Up/Dn  Move cursor\n\
                 Enter  Open mode\n\
                 q      Quit"
            }
            AppMode::TypewriterDone => {
                "SESSION DONE HELP\n\n\
                 F1     Menu\n\
                 F4     Discard & back\n\n\
                 s      Save as document\n\
                 q      Discard & back"
            }
            AppMode::JournalSearch => {
                "JOURNAL SEARCH HELP\n\n\
                 F1     Menu\n\
                 F4     Back to journal\n\n\
                 Type   Enter query\n\
                 Enter  Search\n\
                 Bksp   Delete char\n\
                 q      Back (empty query)"
            }
            AppMode::FileMenu => {
                "FILE MENU HELP\n\n\
                 F1     Menu\n\
                 F4     Back to editor\n\n\
                 Up/Dn  Move cursor\n\
                 Enter  Select action\n\
                 q      Back to editor"
            }
            AppMode::ExportMenu => {
                "EXPORT MENU HELP\n\n\
                 F1     Menu\n\
                 F4     Back to editor\n\n\
                 Up/Dn  Move cursor\n\
                 Enter  Export\n\
                 q      Back to editor"
            }
            _ => {
                "HELP\n\n\
                 F1     Menu\n\
                 F4     Back\n\n\
                 Press any key\n\
                 to close."
            }
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
