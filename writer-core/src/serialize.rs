#[derive(Clone, Debug, PartialEq)]
pub struct WriterConfig {
    pub default_mode: u8,      // 0=editor, 1=journal, 2=typewriter
    pub autosave: bool,
    pub show_line_numbers: bool,
}

impl WriterConfig {
    pub fn default() -> Self {
        Self {
            default_mode: 0,
            autosave: true,
            show_line_numbers: false,
        }
    }
}

/// Serialize a document: [u16 title_len][title_utf8][content_utf8...]
pub fn serialize_document(title: &str, content: &str) -> Vec<u8> {
    let title_bytes = title.as_bytes();
    let title_len = title_bytes.len() as u16;
    let content_bytes = content.as_bytes();

    let mut data = Vec::with_capacity(2 + title_bytes.len() + content_bytes.len());
    data.extend_from_slice(&title_len.to_le_bytes());
    data.extend_from_slice(title_bytes);
    data.extend_from_slice(content_bytes);
    data
}

/// Deserialize a document: returns (title, content)
pub fn deserialize_document(bytes: &[u8]) -> Option<(String, String)> {
    if bytes.len() < 2 {
        return None;
    }
    let title_len = u16::from_le_bytes(bytes[0..2].try_into().ok()?) as usize;
    if bytes.len() < 2 + title_len {
        return None;
    }
    let title = String::from_utf8_lossy(&bytes[2..2 + title_len]).to_string();
    let content = String::from_utf8_lossy(&bytes[2 + title_len..]).to_string();
    Some((title, content))
}

/// Serialize config: [u8 default_mode][u8 autosave][u8 show_line_numbers]
pub fn serialize_config(config: &WriterConfig) -> Vec<u8> {
    vec![
        config.default_mode,
        config.autosave as u8,
        config.show_line_numbers as u8,
    ]
}

/// Deserialize config
pub fn deserialize_config(bytes: &[u8]) -> Option<WriterConfig> {
    if bytes.len() < 3 {
        return None;
    }
    Some(WriterConfig {
        default_mode: bytes[0],
        autosave: bytes[1] != 0,
        show_line_numbers: bytes[2] != 0,
    })
}

/// Serialize a document index: [u32 count][u16 name_len][name_utf8]...
pub fn serialize_index(names: &[String]) -> Vec<u8> {
    let mut data = Vec::new();
    let count = names.len() as u32;
    data.extend_from_slice(&count.to_le_bytes());
    for name in names {
        let name_bytes = name.as_bytes();
        let name_len = name_bytes.len() as u16;
        data.extend_from_slice(&name_len.to_le_bytes());
        data.extend_from_slice(name_bytes);
    }
    data
}

/// Deserialize a document index
pub fn deserialize_index(bytes: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    if bytes.len() < 4 {
        return names;
    }
    let count = u32::from_le_bytes(bytes[0..4].try_into().unwrap_or([0; 4])) as usize;
    let mut offset = 4;
    for _ in 0..count {
        if offset + 2 > bytes.len() {
            break;
        }
        let name_len = u16::from_le_bytes(
            bytes[offset..offset + 2].try_into().unwrap_or([0; 2])
        ) as usize;
        offset += 2;
        if offset + name_len > bytes.len() {
            break;
        }
        let name = String::from_utf8_lossy(&bytes[offset..offset + name_len]).to_string();
        offset += name_len;
        names.push(name);
    }
    names
}

/// Convert epoch milliseconds to a date string (YYYY-MM-DD)
pub fn epoch_ms_to_date(epoch_ms: u64) -> String {
    let total_seconds = epoch_ms / 1000;
    let mut days = (total_seconds / 86400) as i64;

    // Calculate year, month, day from days since epoch (1970-01-01)
    let mut year = 1970i32;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    let days_in_months: [i64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 0u32;
    for (i, &dim) in days_in_months.iter().enumerate() {
        if days < dim {
            month = i as u32 + 1;
            break;
        }
        days -= dim;
    }
    if month == 0 {
        month = 12;
    }
    let day = days as u32 + 1;

    format!("{:04}-{:02}-{:02}", year, month, day)
}

/// Get day-of-week abbreviation from epoch ms (0=Thu for 1970-01-01)
pub fn epoch_ms_to_weekday(epoch_ms: u64) -> &'static str {
    let days = (epoch_ms / 1000 / 86400) as u64;
    // 1970-01-01 was a Thursday (index 4)
    let weekday = (days + 4) % 7;
    match weekday {
        0 => "Sun",
        1 => "Mon",
        2 => "Tue",
        3 => "Wed",
        4 => "Thu",
        5 => "Fri",
        6 => "Sat",
        _ => "???",
    }
}

/// Parse a date string (YYYY-MM-DD) to epoch ms (midnight UTC)
pub fn date_to_epoch_ms(date: &str) -> Option<u64> {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;

    if month < 1 || month > 12 || day < 1 || day > 31 {
        return None;
    }

    // Count days from 1970-01-01
    let mut total_days: u64 = 0;
    for y in 1970..year {
        total_days += if is_leap_year(y) { 366 } else { 365 };
    }

    let days_in_months: [u64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    for i in 0..(month as usize - 1) {
        total_days += days_in_months[i];
    }
    total_days += (day - 1) as u64;

    Some(total_days * 86400 * 1000)
}

/// Navigate to previous day from a date string
pub fn prev_day(date: &str) -> String {
    if let Some(ms) = date_to_epoch_ms(date) {
        if ms >= 86400 * 1000 {
            epoch_ms_to_date(ms - 86400 * 1000)
        } else {
            date.to_string()
        }
    } else {
        date.to_string()
    }
}

/// Navigate to next day from a date string
pub fn next_day(date: &str) -> String {
    if let Some(ms) = date_to_epoch_ms(date) {
        epoch_ms_to_date(ms + 86400 * 1000)
    } else {
        date.to_string()
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize_document() {
        let data = serialize_document("My Doc", "Hello\nWorld");
        let (title, content) = deserialize_document(&data).unwrap();
        assert_eq!(title, "My Doc");
        assert_eq!(content, "Hello\nWorld");
    }

    #[test]
    fn test_serialize_deserialize_config() {
        let config = WriterConfig {
            default_mode: 1,
            autosave: true,
            show_line_numbers: false,
        };
        let data = serialize_config(&config);
        let restored = deserialize_config(&data).unwrap();
        assert_eq!(restored.default_mode, 1);
        assert!(restored.autosave);
        assert!(!restored.show_line_numbers);
    }

    #[test]
    fn test_serialize_deserialize_index() {
        let names = vec!["doc1".to_string(), "my notes".to_string()];
        let data = serialize_index(&names);
        let restored = deserialize_index(&data);
        assert_eq!(restored, names);
    }

    #[test]
    fn test_empty_index() {
        let names: Vec<String> = vec![];
        let data = serialize_index(&names);
        let restored = deserialize_index(&data);
        assert!(restored.is_empty());
    }

    #[test]
    fn test_epoch_ms_to_date() {
        // 2026-01-23 = days since epoch
        // Known: 2026-01-23 midnight UTC
        assert_eq!(epoch_ms_to_date(0), "1970-01-01");
        assert_eq!(epoch_ms_to_date(86400 * 1000), "1970-01-02");
    }

    #[test]
    fn test_date_to_epoch_and_back() {
        let date = "2026-01-23";
        let ms = date_to_epoch_ms(date).unwrap();
        let back = epoch_ms_to_date(ms);
        assert_eq!(back, date);
    }

    #[test]
    fn test_prev_next_day() {
        assert_eq!(next_day("2026-01-23"), "2026-01-24");
        assert_eq!(prev_day("2026-01-23"), "2026-01-22");
        assert_eq!(next_day("2026-01-31"), "2026-02-01");
        assert_eq!(prev_day("2026-02-01"), "2026-01-31");
    }

    #[test]
    fn test_weekday() {
        // 1970-01-01 was Thursday
        assert_eq!(epoch_ms_to_weekday(0), "Thu");
    }

    #[test]
    fn test_leap_year() {
        assert!(is_leap_year(2000));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(1900));
        assert!(!is_leap_year(2023));
    }

    #[test]
    fn test_deserialize_document_too_short() {
        assert_eq!(deserialize_document(&[0]), None);
        assert_eq!(deserialize_document(&[5, 0]), None); // title_len=5 but only 2 bytes
    }

    #[test]
    fn test_deserialize_config_too_short() {
        assert_eq!(deserialize_config(&[0, 1]), None);
    }
}
