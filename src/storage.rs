use std::io::{Read, Write, Seek, SeekFrom};
use writer_core::serialize::{
    serialize_document, deserialize_document,
    serialize_index, deserialize_index,
};

const DICT_DOCS: &str = "writer.docs";
const DICT_JOURNAL: &str = "writer.journal";
const INDEX_KEY: &str = "_index";

pub struct WriterStorage {
    pddb: pddb::Pddb,
}

impl WriterStorage {
    pub fn new() -> Self {
        let pddb = pddb::Pddb::new();
        pddb.try_mount();
        Self { pddb }
    }

    // ---- Document Operations ----

    pub fn list_docs(&self) -> Vec<String> {
        match self.pddb.get(DICT_DOCS, INDEX_KEY, None, false, false, None, None::<fn()>) {
            Ok(mut key) => {
                let mut data = Vec::new();
                key.seek(SeekFrom::Start(0)).ok();
                if key.read_to_end(&mut data).is_ok() && data.len() >= 4 {
                    deserialize_index(&data)
                } else {
                    Vec::new()
                }
            }
            Err(_) => Vec::new(),
        }
    }

    pub fn save_doc(&self, name: &str, content: &str) {
        let key_name = format!("doc_{}", name);
        let data = serialize_document(name, content);

        match self.pddb.get(DICT_DOCS, &key_name, None, true, true, Some(data.len()), None::<fn()>) {
            Ok(mut key) => {
                key.seek(SeekFrom::Start(0)).ok();
                key.write_all(&data).ok();
            }
            Err(e) => {
                log::error!("Failed to save doc '{}': {:?}", name, e);
                return;
            }
        }

        // Update index
        let mut names = self.list_docs();
        if !names.iter().any(|n| n == name) {
            names.push(name.to_string());
            self.write_doc_index(&names);
        }

        self.pddb.sync().ok();
    }

    pub fn load_doc(&self, name: &str) -> Option<String> {
        let key_name = format!("doc_{}", name);
        match self.pddb.get(DICT_DOCS, &key_name, None, false, false, None, None::<fn()>) {
            Ok(mut key) => {
                let mut data = Vec::new();
                key.seek(SeekFrom::Start(0)).ok();
                if key.read_to_end(&mut data).is_ok() && !data.is_empty() {
                    deserialize_document(&data).map(|(_, content)| content)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    pub fn delete_doc(&self, name: &str) {
        let key_name = format!("doc_{}", name);
        self.pddb.delete_key(DICT_DOCS, &key_name, None).ok();

        // Update index
        let mut names = self.list_docs();
        names.retain(|n| n != name);
        self.write_doc_index(&names);

        self.pddb.sync().ok();
    }

    pub fn next_doc_name(&self, prefix: &str) -> String {
        let existing = self.list_docs();
        let mut n = 1u32;
        loop {
            let candidate = if n == 1 {
                prefix.to_string()
            } else {
                format!("{} {}", prefix, n)
            };
            if !existing.iter().any(|name| name == &candidate) {
                return candidate;
            }
            n += 1;
            if n > 999 {
                return format!("{} {}", prefix, n);
            }
        }
    }

    fn write_doc_index(&self, names: &[String]) {
        let data = serialize_index(names);
        match self.pddb.get(DICT_DOCS, INDEX_KEY, None, true, true, Some(data.len()), None::<fn()>) {
            Ok(mut key) => {
                key.seek(SeekFrom::Start(0)).ok();
                key.write_all(&data).ok();
            }
            Err(e) => log::error!("Failed to write doc index: {:?}", e),
        }
    }

    // ---- Journal Operations ----

    pub fn load_journal_entry(&self, date: &str) -> Option<String> {
        match self.pddb.get(DICT_JOURNAL, date, None, false, false, None, None::<fn()>) {
            Ok(mut key) => {
                let mut content = String::new();
                key.seek(SeekFrom::Start(0)).ok();
                if key.read_to_string(&mut content).is_ok() && !content.is_empty() {
                    Some(content)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    pub fn save_journal_entry(&self, date: &str, content: &str) {
        let data = content.as_bytes();
        match self.pddb.get(DICT_JOURNAL, date, None, true, true, Some(data.len()), None::<fn()>) {
            Ok(mut key) => {
                key.seek(SeekFrom::Start(0)).ok();
                key.write_all(data).ok();
            }
            Err(e) => {
                log::error!("Failed to save journal entry for {}: {:?}", date, e);
                return;
            }
        }

        // Update journal index
        let mut dates = self.list_journal_dates();
        if !dates.iter().any(|d| d == date) {
            dates.push(date.to_string());
            dates.sort();
            self.write_journal_index(&dates);
        }

        self.pddb.sync().ok();
    }

    pub fn list_journal_dates(&self) -> Vec<String> {
        match self.pddb.get(DICT_JOURNAL, INDEX_KEY, None, false, false, None, None::<fn()>) {
            Ok(mut key) => {
                let mut data = String::new();
                key.seek(SeekFrom::Start(0)).ok();
                if key.read_to_string(&mut data).is_ok() {
                    data.lines()
                        .filter(|l| !l.is_empty())
                        .map(|l| l.to_string())
                        .collect()
                } else {
                    Vec::new()
                }
            }
            Err(_) => Vec::new(),
        }
    }

    fn write_journal_index(&self, dates: &[String]) {
        let data = dates.join("\n");
        match self.pddb.get(DICT_JOURNAL, INDEX_KEY, None, true, true, Some(data.len()), None::<fn()>) {
            Ok(mut key) => {
                key.seek(SeekFrom::Start(0)).ok();
                key.write_all(data.as_bytes()).ok();
            }
            Err(e) => log::error!("Failed to write journal index: {:?}", e),
        }
    }
}
