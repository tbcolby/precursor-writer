use writer_core::{TextBuffer, serialize::{epoch_ms_to_date, prev_day, next_day}};
use crate::storage::WriterStorage;

#[derive(Clone, Debug)]
pub struct JournalState {
    pub buffer: TextBuffer,
    pub current_date: String,
    pub search_query: String,
    pub search_results: Vec<(String, String)>, // (date, matching line)
}

impl JournalState {
    pub fn new() -> Self {
        Self {
            buffer: TextBuffer::new(),
            current_date: String::new(),
            search_query: String::new(),
            search_results: Vec::new(),
        }
    }

    pub fn jump_to_today(&mut self) {
        // Get current time from system
        // In Xous, we'd use llio::LocalTime, but for initialization
        // we'll set a date that gets updated on first redraw
        let now_ms = get_current_time_ms();
        self.current_date = epoch_ms_to_date(now_ms);
    }

    pub fn load_entry(&mut self, storage: &WriterStorage) {
        if let Some(content) = storage.load_journal_entry(&self.current_date) {
            self.buffer = TextBuffer::from_text(&content);
        } else {
            self.buffer = TextBuffer::new();
        }
        self.buffer.modified = false;
    }

    pub fn save_entry(&self, storage: &WriterStorage) {
        if self.buffer.modified || self.buffer.word_count() > 0 {
            let content = self.buffer.to_string();
            storage.save_journal_entry(&self.current_date, &content);
        }
    }

    pub fn prev_day(&mut self, storage: &WriterStorage) {
        self.current_date = prev_day(&self.current_date);
        self.load_entry(storage);
    }

    pub fn next_day(&mut self, storage: &WriterStorage) {
        self.current_date = next_day(&self.current_date);
        self.load_entry(storage);
    }

    pub fn search_entries(&mut self, storage: &WriterStorage) {
        self.search_results.clear();
        if self.search_query.is_empty() {
            return;
        }
        let query = self.search_query.to_lowercase();
        let dates = storage.list_journal_dates();
        for date in dates {
            if let Some(content) = storage.load_journal_entry(&date) {
                for line in content.lines() {
                    if line.to_lowercase().contains(&query) {
                        self.search_results.push((date.clone(), line.to_string()));
                        if self.search_results.len() >= 10 {
                            return;
                        }
                        break; // One match per date
                    }
                }
            }
        }
    }
}

/// Get current epoch milliseconds using llio::LocalTime
pub fn get_current_time_ms() -> u64 {
    let mut lt = llio::LocalTime::new();
    lt.get_local_time_ms().unwrap_or(0)
}
