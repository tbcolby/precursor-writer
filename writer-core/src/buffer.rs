#[derive(Clone, Debug)]
pub struct Cursor {
    pub line: usize,
    pub col: usize,
}

impl Cursor {
    pub fn new() -> Self {
        Self { line: 0, col: 0 }
    }
}

#[derive(Clone, Debug)]
pub struct TextBuffer {
    pub lines: Vec<String>,
    pub cursor: Cursor,
    pub viewport_top: usize,
    pub viewport_lines: usize,
    pub modified: bool,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor: Cursor::new(),
            viewport_top: 0,
            viewport_lines: 13,
            modified: false,
        }
    }

    pub fn from_text(text: &str) -> Self {
        let lines: Vec<String> = if text.is_empty() {
            vec![String::new()]
        } else {
            text.lines().map(|l| l.to_string()).collect()
        };
        // Ensure at least one line
        let lines = if lines.is_empty() { vec![String::new()] } else { lines };
        Self {
            lines,
            cursor: Cursor::new(),
            viewport_top: 0,
            viewport_lines: 13,
            modified: false,
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        let line = &mut self.lines[self.cursor.line];
        if self.cursor.col >= line.len() {
            line.push(ch);
        } else {
            line.insert(self.cursor.col, ch);
        }
        self.cursor.col += 1;
        self.modified = true;
    }

    pub fn delete_back(&mut self) {
        if self.cursor.col > 0 {
            let line = &mut self.lines[self.cursor.line];
            self.cursor.col -= 1;
            line.remove(self.cursor.col);
            self.modified = true;
        } else if self.cursor.line > 0 {
            // Merge with previous line
            let current = self.lines.remove(self.cursor.line);
            self.cursor.line -= 1;
            self.cursor.col = self.lines[self.cursor.line].len();
            self.lines[self.cursor.line].push_str(&current);
            self.modified = true;
        }
        self.ensure_cursor_visible();
    }

    pub fn delete_forward(&mut self) {
        let line_len = self.lines[self.cursor.line].len();
        if self.cursor.col < line_len {
            self.lines[self.cursor.line].remove(self.cursor.col);
            self.modified = true;
        } else if self.cursor.line + 1 < self.lines.len() {
            // Merge next line into current
            let next = self.lines.remove(self.cursor.line + 1);
            self.lines[self.cursor.line].push_str(&next);
            self.modified = true;
        }
    }

    pub fn newline(&mut self) {
        let line = &self.lines[self.cursor.line];
        let remainder = line[self.cursor.col..].to_string();
        self.lines[self.cursor.line].truncate(self.cursor.col);
        self.cursor.line += 1;
        self.cursor.col = 0;
        self.lines.insert(self.cursor.line, remainder);
        self.modified = true;
        self.ensure_cursor_visible();
    }

    pub fn move_up(&mut self) {
        if self.cursor.line > 0 {
            self.cursor.line -= 1;
            let line_len = self.lines[self.cursor.line].len();
            if self.cursor.col > line_len {
                self.cursor.col = line_len;
            }
            self.ensure_cursor_visible();
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor.line + 1 < self.lines.len() {
            self.cursor.line += 1;
            let line_len = self.lines[self.cursor.line].len();
            if self.cursor.col > line_len {
                self.cursor.col = line_len;
            }
            self.ensure_cursor_visible();
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.cursor.col = self.lines[self.cursor.line].len();
            self.ensure_cursor_visible();
        }
    }

    pub fn move_right(&mut self) {
        let line_len = self.lines[self.cursor.line].len();
        if self.cursor.col < line_len {
            self.cursor.col += 1;
        } else if self.cursor.line + 1 < self.lines.len() {
            self.cursor.line += 1;
            self.cursor.col = 0;
            self.ensure_cursor_visible();
        }
    }

    pub fn move_home(&mut self) {
        self.cursor.col = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor.col = self.lines[self.cursor.line].len();
    }

    pub fn to_string(&self) -> String {
        self.lines.join("\n")
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn word_count(&self) -> usize {
        self.lines.iter()
            .flat_map(|l| l.split_whitespace())
            .count()
    }

    pub fn char_count(&self) -> usize {
        self.lines.iter()
            .map(|l| l.len())
            .sum::<usize>()
            + self.lines.len().saturating_sub(1) // count newlines
    }

    pub fn ensure_cursor_visible(&mut self) {
        if self.cursor.line < self.viewport_top {
            self.viewport_top = self.cursor.line;
        } else if self.cursor.line >= self.viewport_top + self.viewport_lines {
            self.viewport_top = self.cursor.line - self.viewport_lines + 1;
        }
    }

    /// Append a character at the end of the buffer (for typewriter mode)
    pub fn append_char(&mut self, ch: char) {
        let last = self.lines.len() - 1;
        self.lines[last].push(ch);
        self.cursor.line = last;
        self.cursor.col = self.lines[last].len();
        self.modified = true;
        self.ensure_cursor_visible();
    }

    /// Append a newline at the end (for typewriter mode)
    pub fn append_newline(&mut self) {
        self.lines.push(String::new());
        self.cursor.line = self.lines.len() - 1;
        self.cursor.col = 0;
        self.modified = true;
        self.ensure_cursor_visible();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer() {
        let buf = TextBuffer::new();
        assert_eq!(buf.lines.len(), 1);
        assert_eq!(buf.cursor.line, 0);
        assert_eq!(buf.cursor.col, 0);
        assert!(!buf.modified);
    }

    #[test]
    fn test_from_text() {
        let buf = TextBuffer::from_text("hello\nworld");
        assert_eq!(buf.lines.len(), 2);
        assert_eq!(buf.lines[0], "hello");
        assert_eq!(buf.lines[1], "world");
    }

    #[test]
    fn test_insert_char() {
        let mut buf = TextBuffer::new();
        buf.insert_char('h');
        buf.insert_char('i');
        assert_eq!(buf.lines[0], "hi");
        assert_eq!(buf.cursor.col, 2);
        assert!(buf.modified);
    }

    #[test]
    fn test_delete_back() {
        let mut buf = TextBuffer::from_text("hello");
        buf.cursor.col = 5;
        buf.delete_back();
        assert_eq!(buf.lines[0], "hell");
        assert_eq!(buf.cursor.col, 4);
    }

    #[test]
    fn test_delete_back_merge_lines() {
        let mut buf = TextBuffer::from_text("hello\nworld");
        buf.cursor.line = 1;
        buf.cursor.col = 0;
        buf.delete_back();
        assert_eq!(buf.lines.len(), 1);
        assert_eq!(buf.lines[0], "helloworld");
        assert_eq!(buf.cursor.line, 0);
        assert_eq!(buf.cursor.col, 5);
    }

    #[test]
    fn test_newline() {
        let mut buf = TextBuffer::from_text("hello");
        buf.cursor.col = 3;
        buf.newline();
        assert_eq!(buf.lines.len(), 2);
        assert_eq!(buf.lines[0], "hel");
        assert_eq!(buf.lines[1], "lo");
        assert_eq!(buf.cursor.line, 1);
        assert_eq!(buf.cursor.col, 0);
    }

    #[test]
    fn test_cursor_movement() {
        let mut buf = TextBuffer::from_text("hello\nworld");
        buf.cursor.col = 2;
        buf.move_down();
        assert_eq!(buf.cursor.line, 1);
        assert_eq!(buf.cursor.col, 2);
        buf.move_up();
        assert_eq!(buf.cursor.line, 0);
        buf.move_end();
        assert_eq!(buf.cursor.col, 5);
        buf.move_home();
        assert_eq!(buf.cursor.col, 0);
    }

    #[test]
    fn test_word_count() {
        let buf = TextBuffer::from_text("hello world\nfoo bar baz");
        assert_eq!(buf.word_count(), 5);
    }

    #[test]
    fn test_char_count() {
        let buf = TextBuffer::from_text("hi\nbye");
        // "hi" (2) + "\n" (1) + "bye" (3) = 6
        assert_eq!(buf.char_count(), 6);
    }

    #[test]
    fn test_viewport_scrolling() {
        let mut buf = TextBuffer::new();
        buf.viewport_lines = 3;
        for i in 0..10 {
            buf.lines.push(format!("line {}", i));
        }
        buf.cursor.line = 5;
        buf.ensure_cursor_visible();
        assert_eq!(buf.viewport_top, 3);
    }

    #[test]
    fn test_delete_forward() {
        let mut buf = TextBuffer::from_text("hello");
        buf.cursor.col = 2;
        buf.delete_forward();
        assert_eq!(buf.lines[0], "helo");
    }

    #[test]
    fn test_delete_forward_merge() {
        let mut buf = TextBuffer::from_text("hello\nworld");
        buf.cursor.col = 5;
        buf.delete_forward();
        assert_eq!(buf.lines.len(), 1);
        assert_eq!(buf.lines[0], "helloworld");
    }

    #[test]
    fn test_append_char() {
        let mut buf = TextBuffer::new();
        buf.append_char('a');
        buf.append_char('b');
        assert_eq!(buf.lines[0], "ab");
        assert_eq!(buf.cursor.col, 2);
    }

    #[test]
    fn test_append_newline() {
        let mut buf = TextBuffer::new();
        buf.append_char('a');
        buf.append_newline();
        buf.append_char('b');
        assert_eq!(buf.lines.len(), 2);
        assert_eq!(buf.lines[0], "a");
        assert_eq!(buf.lines[1], "b");
    }

    #[test]
    fn test_move_right_wraps() {
        let mut buf = TextBuffer::from_text("ab\ncd");
        buf.cursor.col = 2; // end of first line
        buf.move_right();
        assert_eq!(buf.cursor.line, 1);
        assert_eq!(buf.cursor.col, 0);
    }

    #[test]
    fn test_move_left_wraps() {
        let mut buf = TextBuffer::from_text("ab\ncd");
        buf.cursor.line = 1;
        buf.cursor.col = 0;
        buf.move_left();
        assert_eq!(buf.cursor.line, 0);
        assert_eq!(buf.cursor.col, 2);
    }
}
