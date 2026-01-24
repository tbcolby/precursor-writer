use writer_core::TextBuffer;

#[derive(Clone, Debug)]
pub struct EditorState {
    pub buffer: TextBuffer,
    pub doc_name: String,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            buffer: TextBuffer::new(),
            doc_name: String::new(),
        }
    }

    pub fn with_name(name: &str) -> Self {
        Self {
            buffer: TextBuffer::new(),
            doc_name: name.to_string(),
        }
    }

    pub fn with_content(name: &str, content: &str) -> Self {
        Self {
            buffer: TextBuffer::from_text(content),
            doc_name: name.to_string(),
        }
    }
}
