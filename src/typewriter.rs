use writer_core::TextBuffer;

#[derive(Clone, Debug)]
pub struct TypewriterState {
    pub buffer: TextBuffer,
}

impl TypewriterState {
    pub fn new() -> Self {
        Self {
            buffer: TextBuffer::new(),
        }
    }
}
