#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LineKind {
    Normal,
    Heading1,
    Heading2,
    Heading3,
    CodeBlock,
    BlockQuote,
    UnorderedList,
    OrderedList,
    HorizontalRule,
    Empty,
}

impl LineKind {
    pub fn classify(line: &str) -> Self {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            return LineKind::Empty;
        }

        // Horizontal rule: ---, ***, ___  (3+ chars, optionally with spaces)
        if is_horizontal_rule(trimmed) {
            return LineKind::HorizontalRule;
        }

        // Code block fence: ```
        if trimmed.starts_with("```") {
            return LineKind::CodeBlock;
        }

        // Indented code block (4+ spaces or tab)
        if line.starts_with("    ") || line.starts_with('\t') {
            return LineKind::CodeBlock;
        }

        // Headings
        if trimmed.starts_with("### ") {
            return LineKind::Heading3;
        }
        if trimmed.starts_with("## ") {
            return LineKind::Heading2;
        }
        if trimmed.starts_with("# ") {
            return LineKind::Heading1;
        }

        // Block quote
        if trimmed.starts_with("> ") || trimmed == ">" {
            return LineKind::BlockQuote;
        }

        // Unordered list
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            return LineKind::UnorderedList;
        }

        // Ordered list: digit(s) followed by ". "
        if is_ordered_list(trimmed) {
            return LineKind::OrderedList;
        }

        LineKind::Normal
    }

    /// Strip the markdown prefix from a line, returning the content portion.
    pub fn strip_prefix(line: &str, kind: LineKind) -> &str {
        let trimmed = line.trim_start();
        match kind {
            LineKind::Heading1 => trimmed.strip_prefix("# ").unwrap_or(trimmed),
            LineKind::Heading2 => trimmed.strip_prefix("## ").unwrap_or(trimmed),
            LineKind::Heading3 => trimmed.strip_prefix("### ").unwrap_or(trimmed),
            LineKind::BlockQuote => {
                if let Some(rest) = trimmed.strip_prefix("> ") {
                    rest
                } else if trimmed == ">" {
                    ""
                } else {
                    trimmed
                }
            }
            LineKind::UnorderedList => {
                if let Some(rest) = trimmed.strip_prefix("- ") {
                    rest
                } else if let Some(rest) = trimmed.strip_prefix("* ") {
                    rest
                } else {
                    trimmed
                }
            }
            LineKind::OrderedList => {
                // Strip "N. " prefix
                if let Some(dot_pos) = trimmed.find(". ") {
                    let prefix = &trimmed[..dot_pos];
                    if prefix.chars().all(|c| c.is_ascii_digit()) {
                        &trimmed[dot_pos + 2..]
                    } else {
                        trimmed
                    }
                } else {
                    trimmed
                }
            }
            LineKind::CodeBlock => {
                if trimmed.starts_with("```") {
                    ""
                } else if line.starts_with("    ") {
                    &line[4..]
                } else if line.starts_with('\t') {
                    &line[1..]
                } else {
                    line
                }
            }
            LineKind::HorizontalRule => "",
            LineKind::Empty => "",
            LineKind::Normal => line,
        }
    }
}

fn is_horizontal_rule(s: &str) -> bool {
    let chars: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
    if chars.len() < 3 {
        return false;
    }
    let first = chars[0];
    if first != '-' && first != '*' && first != '_' {
        return false;
    }
    chars.iter().all(|&c| c == first)
}

fn is_ordered_list(s: &str) -> bool {
    if let Some(dot_pos) = s.find(". ") {
        let prefix = &s[..dot_pos];
        !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit())
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_empty() {
        assert_eq!(LineKind::classify(""), LineKind::Empty);
        assert_eq!(LineKind::classify("   "), LineKind::Empty);
    }

    #[test]
    fn test_classify_headings() {
        assert_eq!(LineKind::classify("# Title"), LineKind::Heading1);
        assert_eq!(LineKind::classify("## Subtitle"), LineKind::Heading2);
        assert_eq!(LineKind::classify("### Section"), LineKind::Heading3);
    }

    #[test]
    fn test_classify_code_block() {
        assert_eq!(LineKind::classify("```rust"), LineKind::CodeBlock);
        assert_eq!(LineKind::classify("```"), LineKind::CodeBlock);
        assert_eq!(LineKind::classify("    code here"), LineKind::CodeBlock);
        assert_eq!(LineKind::classify("\tcode here"), LineKind::CodeBlock);
    }

    #[test]
    fn test_classify_block_quote() {
        assert_eq!(LineKind::classify("> quote"), LineKind::BlockQuote);
        assert_eq!(LineKind::classify(">"), LineKind::BlockQuote);
    }

    #[test]
    fn test_classify_lists() {
        assert_eq!(LineKind::classify("- item"), LineKind::UnorderedList);
        assert_eq!(LineKind::classify("* item"), LineKind::UnorderedList);
        assert_eq!(LineKind::classify("1. first"), LineKind::OrderedList);
        assert_eq!(LineKind::classify("12. twelfth"), LineKind::OrderedList);
    }

    #[test]
    fn test_classify_horizontal_rule() {
        assert_eq!(LineKind::classify("---"), LineKind::HorizontalRule);
        assert_eq!(LineKind::classify("***"), LineKind::HorizontalRule);
        assert_eq!(LineKind::classify("___"), LineKind::HorizontalRule);
        assert_eq!(LineKind::classify("- - -"), LineKind::HorizontalRule);
    }

    #[test]
    fn test_classify_normal() {
        assert_eq!(LineKind::classify("hello world"), LineKind::Normal);
        assert_eq!(LineKind::classify("just text"), LineKind::Normal);
    }

    #[test]
    fn test_strip_prefix_heading() {
        assert_eq!(LineKind::strip_prefix("# Title", LineKind::Heading1), "Title");
        assert_eq!(LineKind::strip_prefix("## Sub", LineKind::Heading2), "Sub");
        assert_eq!(LineKind::strip_prefix("### Sec", LineKind::Heading3), "Sec");
    }

    #[test]
    fn test_strip_prefix_quote() {
        assert_eq!(LineKind::strip_prefix("> text", LineKind::BlockQuote), "text");
        assert_eq!(LineKind::strip_prefix(">", LineKind::BlockQuote), "");
    }

    #[test]
    fn test_strip_prefix_list() {
        assert_eq!(LineKind::strip_prefix("- item", LineKind::UnorderedList), "item");
        assert_eq!(LineKind::strip_prefix("* item", LineKind::UnorderedList), "item");
        assert_eq!(LineKind::strip_prefix("1. first", LineKind::OrderedList), "first");
    }

    #[test]
    fn test_strip_prefix_code() {
        assert_eq!(LineKind::strip_prefix("    code", LineKind::CodeBlock), "code");
        assert_eq!(LineKind::strip_prefix("\tcode", LineKind::CodeBlock), "code");
        assert_eq!(LineKind::strip_prefix("```rust", LineKind::CodeBlock), "");
    }

    #[test]
    fn test_strip_prefix_normal() {
        assert_eq!(LineKind::strip_prefix("hello", LineKind::Normal), "hello");
    }

    #[test]
    fn test_not_heading_without_space() {
        assert_eq!(LineKind::classify("#nospace"), LineKind::Normal);
        assert_eq!(LineKind::classify("##nospace"), LineKind::Normal);
    }
}
