// Shared UI constants and helpers for the Writer app

/// Truncate a string to fit within a character limit, adding "..." if needed
pub fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        s.to_string()
    } else if max_chars > 3 {
        format!("{}...", &s[..max_chars - 3])
    } else {
        s[..max_chars].to_string()
    }
}

/// Format a number with comma separators (for display)
pub fn format_number(n: usize) -> String {
    if n < 1000 {
        return n.to_string();
    }
    let s = n.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello world", 8), "hello...");
        assert_eq!(truncate_str("hi", 2), "hi");
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(42), "42");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1847), "1,847");
        assert_eq!(format_number(1000000), "1,000,000");
    }

}
