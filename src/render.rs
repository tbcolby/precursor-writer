use std::fmt::Write;
use gam::{Gam, GlyphStyle, Gid};
use gam::menu::*;
use writer_core::{TextBuffer, LineKind};
use writer_core::serialize::{date_to_epoch_ms, epoch_ms_to_weekday};
use crate::ui::{format_number, truncate_str};

const MARGIN_LEFT: isize = 8;
const MARGIN_RIGHT: isize = 8;
const STATUS_BAR_HEIGHT: isize = 28;
const LINE_HEIGHT_REGULAR: isize = 18;
const LINE_HEIGHT_LARGE: isize = 28;

pub struct Renderer {
    gam: Gam,
    content: Gid,
    screensize: Point,
}

impl Renderer {
    pub fn new(gam: Gam, content: Gid, screensize: Point) -> Self {
        Self { gam, content, screensize }
    }

    fn clear(&self) {
        self.gam.draw_rectangle(
            self.content,
            Rectangle::new_with_style(
                Point::new(0, 0),
                self.screensize,
                DrawStyle {
                    fill_color: Some(PixelColor::Light),
                    stroke_color: None,
                    stroke_width: 0,
                },
            ),
        ).expect("can't clear");
    }

    fn post_text(&self, x: isize, y: isize, w: isize, h: isize, style: GlyphStyle, text: &str) {
        let mut tv = TextView::new(
            self.content,
            TextBounds::BoundingBox(Rectangle::new_coords(x, y, x + w, y + h)),
        );
        tv.style = style;
        tv.clear_area = true;
        write!(tv.text, "{}", text).unwrap();
        self.gam.post_textview(&mut tv).expect("can't post text");
    }

    fn finish(&self) {
        self.gam.redraw().expect("can't redraw");
    }

    // ---- Menu Overlay ----

    pub fn draw_menu(&self, items: &[&str], cursor: usize) {
        self.clear();

        self.post_text(
            MARGIN_LEFT, 12,
            self.screensize.x - MARGIN_LEFT * 2, 30,
            GlyphStyle::Bold,
            "MENU",
        );

        let line_height = 30;
        let list_top = 52;

        for (i, item) in items.iter().enumerate() {
            let y = list_top + (i as isize) * line_height;
            let marker = if i == cursor { "> " } else { "  " };
            let label = format!("{}{}", marker, item);
            self.post_text(
                16, y,
                self.screensize.x - 32, line_height - 2,
                GlyphStyle::Regular,
                &label,
            );
        }

        self.post_text(
            MARGIN_LEFT, self.screensize.y - 36,
            self.screensize.x - MARGIN_LEFT * 2, 28,
            GlyphStyle::Small,
            "arrows=select  ENTER=open  F4=close",
        );

        self.finish();
    }

    // ---- Help Screen ----

    pub fn draw_help(&self, help_text: &str) {
        self.clear();

        let line_height = 20;
        let mut y = 16isize;

        for line in help_text.lines() {
            if y + line_height > self.screensize.y - 36 {
                break;
            }
            let style = if y == 16 { GlyphStyle::Bold } else { GlyphStyle::Small };
            self.post_text(
                16, y,
                self.screensize.x - 32, line_height - 2,
                style,
                line,
            );
            y += line_height;
        }

        self.post_text(
            MARGIN_LEFT, self.screensize.y - 28,
            self.screensize.x - MARGIN_LEFT * 2, 22,
            GlyphStyle::Small,
            "Press any key to close",
        );

        self.finish();
    }

    // ---- Confirm Exit ----

    pub fn draw_confirm_exit(&self) {
        self.clear();

        self.post_text(
            MARGIN_LEFT, 40,
            self.screensize.x - MARGIN_LEFT * 2, 30,
            GlyphStyle::Bold,
            "Unsaved Changes",
        );

        self.post_text(
            MARGIN_LEFT, 80,
            self.screensize.x - MARGIN_LEFT * 2, 40,
            GlyphStyle::Regular,
            "Document has unsaved changes.",
        );

        self.post_text(
            20, 140,
            self.screensize.x - 40, 80,
            GlyphStyle::Regular,
            "y = Save & exit\nn = Exit without saving\nF4 = Cancel",
        );

        self.finish();
    }

    // ---- Mode Select ----

    pub fn draw_mode_select(&self, cursor: usize) {
        self.clear();

        // Title
        self.post_text(
            MARGIN_LEFT, 8,
            self.screensize.x - MARGIN_LEFT * 2, 30,
            GlyphStyle::Bold,
            "WRITER",
        );

        // Menu items
        let modes = ["Markdown Editor", "Journal", "Typewriter"];
        let list_top = 60;
        let line_height = 32;

        for (i, mode) in modes.iter().enumerate() {
            let y = list_top + (i as isize) * line_height;
            let marker = if i == cursor { "> " } else { "  " };
            let label = format!("{}{}", marker, mode);
            self.post_text(
                20, y,
                self.screensize.x - 40, line_height - 2,
                GlyphStyle::Regular,
                &label,
            );
        }

        // Footer
        self.post_text(
            MARGIN_LEFT, self.screensize.y - 40,
            self.screensize.x - MARGIN_LEFT * 2, 30,
            GlyphStyle::Small,
            "F1=menu F4=quit  ENTER=open",
        );

        self.finish();
    }

    // ---- Document List ----

    pub fn draw_doc_list(&self, docs: &[String], cursor: usize) {
        self.clear();

        // Title
        self.post_text(
            MARGIN_LEFT, 8,
            self.screensize.x - MARGIN_LEFT * 2, 30,
            GlyphStyle::Bold,
            "DOCUMENTS",
        );

        if docs.is_empty() {
            self.post_text(
                20, 60,
                self.screensize.x - 40, 20,
                GlyphStyle::Regular,
                "No documents yet",
            );
        } else {
            let list_top = 50;
            let line_height = 24;
            let max_visible = ((self.screensize.y - list_top - 50) / line_height) as usize;

            // Determine viewport
            let start = if cursor >= max_visible {
                cursor - max_visible + 1
            } else {
                0
            };

            for (i, doc) in docs.iter().enumerate().skip(start).take(max_visible) {
                let y = list_top + ((i - start) as isize) * line_height;
                let marker = if i == cursor { "> " } else { "  " };
                let label = format!("{}{}", marker, doc);
                self.post_text(
                    16, y,
                    self.screensize.x - 32, line_height - 2,
                    GlyphStyle::Regular,
                    &label,
                );
            }
        }

        // Footer
        self.post_text(
            MARGIN_LEFT, self.screensize.y - 40,
            self.screensize.x - MARGIN_LEFT * 2, 30,
            GlyphStyle::Small,
            "F1=menu F4=back  ENTER=open  n=new  d=del",
        );

        self.finish();
    }

    // ---- Editor ----

    pub fn draw_editor(&self, buffer: &TextBuffer, doc_name: &str, preview: bool, show_line_numbers: bool) {
        self.clear();

        let content_top = 4isize;
        let content_bottom = self.screensize.y - STATUS_BAR_HEIGHT;

        // Render visible lines
        let mut y = content_top;
        let end_line = (buffer.viewport_top + buffer.viewport_lines).min(buffer.lines.len());

        for line_idx in buffer.viewport_top..end_line {
            let line = &buffer.lines[line_idx];
            let kind = LineKind::classify(line);

            let (style, line_h) = match kind {
                LineKind::Heading1 => (GlyphStyle::Large, LINE_HEIGHT_LARGE),
                LineKind::Heading2 | LineKind::Heading3 => (GlyphStyle::Bold, LINE_HEIGHT_REGULAR + 4),
                LineKind::CodeBlock => (GlyphStyle::Monospace, LINE_HEIGHT_REGULAR),
                _ => (GlyphStyle::Regular, LINE_HEIGHT_REGULAR),
            };

            if y + line_h > content_bottom {
                break;
            }

            // Display text
            let display_text = if preview {
                LineKind::strip_prefix(line, kind).to_string()
            } else {
                line.clone()
            };

            // Draw block quote bar
            if kind == LineKind::BlockQuote {
                self.gam.draw_rectangle(
                    self.content,
                    Rectangle::new_with_style(
                        Point::new(MARGIN_LEFT, y + 2),
                        Point::new(MARGIN_LEFT + 3, y + line_h - 2),
                        DrawStyle {
                            fill_color: Some(PixelColor::Dark),
                            stroke_color: None,
                            stroke_width: 0,
                        },
                    ),
                ).ok();
            }

            // Draw horizontal rule
            if kind == LineKind::HorizontalRule {
                let rule_y = y + line_h / 2;
                self.gam.draw_rectangle(
                    self.content,
                    Rectangle::new_with_style(
                        Point::new(MARGIN_LEFT, rule_y),
                        Point::new(self.screensize.x - MARGIN_RIGHT, rule_y + 1),
                        DrawStyle {
                            fill_color: Some(PixelColor::Dark),
                            stroke_color: None,
                            stroke_width: 0,
                        },
                    ),
                ).ok();
                y += line_h;
                continue;
            }

            // Line number column width (4 digits + space = ~40px)
            let line_num_width: isize = if show_line_numbers { 40 } else { 0 };

            // Text offset for block quotes and line numbers
            let text_left = if kind == LineKind::BlockQuote {
                MARGIN_LEFT + line_num_width + 8
            } else {
                MARGIN_LEFT + line_num_width
            };

            // Draw line numbers if enabled
            if show_line_numbers {
                let line_num_str = format!("{:>3} ", line_idx + 1);
                self.post_text(
                    MARGIN_LEFT, y,
                    line_num_width, line_h,
                    GlyphStyle::Monospace,
                    &line_num_str,
                );
            }

            // Render the text line
            if !display_text.is_empty() {
                self.post_text(
                    text_left, y,
                    self.screensize.x - text_left - MARGIN_RIGHT, line_h,
                    style,
                    &display_text,
                );
            }

            // Draw cursor (only in edit mode, after text_left is calculated with line numbers)
            if !preview && line_idx == buffer.cursor.line {
                self.draw_cursor(text_left, y, &display_text, buffer.cursor.col, line_h, style);
            }

            y += line_h;
        }

        // Status bar
        self.draw_status_bar(buffer, doc_name, preview);

        self.finish();
    }

    fn draw_cursor(&self, text_left: isize, y: isize, _line: &str, col: usize, line_h: isize, _style: GlyphStyle) {
        // Approximate character width based on style (monospace-like rendering)
        let char_width: isize = 8; // Approximate for Regular/Monospace
        let cursor_x = text_left + (col as isize) * char_width;
        let cursor_w = char_width.min(3);

        // Draw cursor as a thin dark rectangle
        self.gam.draw_rectangle(
            self.content,
            Rectangle::new_with_style(
                Point::new(cursor_x, y + 1),
                Point::new(cursor_x + cursor_w, y + line_h - 1),
                DrawStyle {
                    fill_color: Some(PixelColor::Dark),
                    stroke_color: None,
                    stroke_width: 0,
                },
            ),
        ).ok();
    }

    fn draw_status_bar(&self, buffer: &TextBuffer, doc_name: &str, preview: bool) {
        let bar_top = self.screensize.y - STATUS_BAR_HEIGHT;

        // Separator line
        self.gam.draw_rectangle(
            self.content,
            Rectangle::new_with_style(
                Point::new(0, bar_top),
                Point::new(self.screensize.x, bar_top + 1),
                DrawStyle {
                    fill_color: Some(PixelColor::Dark),
                    stroke_color: None,
                    stroke_width: 0,
                },
            ),
        ).ok();

        let mode_str = if preview { "PREVIEW" } else { "EDIT" };
        let modified = if buffer.modified { "*" } else { "" };
        let status = format!(
            "{}{} {}:{} W:{}",
            doc_name, modified,
            buffer.cursor.line + 1, buffer.cursor.col + 1,
            buffer.word_count(),
        );

        self.post_text(
            MARGIN_LEFT, bar_top + 4,
            self.screensize.x / 2, STATUS_BAR_HEIGHT - 4,
            GlyphStyle::Small,
            &status,
        );

        self.post_text(
            self.screensize.x / 2, bar_top + 4,
            self.screensize.x / 2 - MARGIN_RIGHT, STATUS_BAR_HEIGHT - 4,
            GlyphStyle::Small,
            mode_str,
        );
    }

    // ---- File Menu ----

    pub fn draw_file_menu(&self, cursor: usize) {
        self.clear();

        self.post_text(
            MARGIN_LEFT, 8,
            self.screensize.x - MARGIN_LEFT * 2, 30,
            GlyphStyle::Bold,
            "FILE",
        );

        let items = ["New Document", "Rename", "Delete Current", "Back to Editor"];
        let list_top = 50;
        let line_height = 32;

        for (i, item) in items.iter().enumerate() {
            let y = list_top + (i as isize) * line_height;
            let marker = if i == cursor { "> " } else { "  " };
            let label = format!("{}{}", marker, item);
            self.post_text(
                20, y,
                self.screensize.x - 40, line_height - 2,
                GlyphStyle::Regular,
                &label,
            );
        }

        self.post_text(
            MARGIN_LEFT, self.screensize.y - 40,
            self.screensize.x - MARGIN_LEFT * 2, 30,
            GlyphStyle::Small,
            "F4=back  ENTER=select",
        );

        self.finish();
    }

    pub fn draw_rename_dialog(&self, new_name: &str, old_name: &str) {
        self.clear();

        self.post_text(
            MARGIN_LEFT, 8,
            self.screensize.x - MARGIN_LEFT * 2, 30,
            GlyphStyle::Bold,
            "RENAME DOCUMENT",
        );

        // Show current name
        let current_label = format!("Current: {}", old_name);
        self.post_text(
            MARGIN_LEFT, 60,
            self.screensize.x - MARGIN_LEFT * 2, 20,
            GlyphStyle::Small,
            &current_label,
        );

        // Input field with cursor
        let input_display = format!("New: {}|", new_name);
        self.post_text(
            MARGIN_LEFT, 100,
            self.screensize.x - MARGIN_LEFT * 2, 24,
            GlyphStyle::Regular,
            &input_display,
        );

        self.post_text(
            MARGIN_LEFT, self.screensize.y - 40,
            self.screensize.x - MARGIN_LEFT * 2, 30,
            GlyphStyle::Small,
            "F4=cancel  ENTER=confirm",
        );

        self.finish();
    }

    // ---- Export Menu ----

    pub fn draw_export_menu(&self, cursor: usize) {
        self.clear();

        self.post_text(
            MARGIN_LEFT, 8,
            self.screensize.x - MARGIN_LEFT * 2, 30,
            GlyphStyle::Bold,
            "EXPORT",
        );

        let items = ["TCP (port 7879)", "USB Keyboard Autotype"];
        let list_top = 60;
        let line_height = 32;

        for (i, item) in items.iter().enumerate() {
            let y = list_top + (i as isize) * line_height;
            let marker = if i == cursor { "> " } else { "  " };
            let label = format!("{}{}", marker, item);
            self.post_text(
                20, y,
                self.screensize.x - 40, line_height - 2,
                GlyphStyle::Regular,
                &label,
            );
        }

        self.post_text(
            MARGIN_LEFT, self.screensize.y - 40,
            self.screensize.x - MARGIN_LEFT * 2, 30,
            GlyphStyle::Small,
            "F4=back  ENTER=select",
        );

        self.finish();
    }

    // ---- Journal ----

    pub fn draw_journal(&self, buffer: &TextBuffer, date: &str) {
        self.clear();

        // Header with date and weekday
        let weekday = date_to_epoch_ms(date)
            .map(epoch_ms_to_weekday)
            .unwrap_or("???");
        let header = format!("JOURNAL  {} {}", date, weekday);
        self.post_text(
            MARGIN_LEFT, 4,
            self.screensize.x - MARGIN_LEFT * 2, 24,
            GlyphStyle::Bold,
            &header,
        );

        // Navigation hint
        self.post_text(
            MARGIN_LEFT, 26,
            self.screensize.x - MARGIN_LEFT * 2, 16,
            GlyphStyle::Small,
            "F1=menu F3=save F4=back  Esc[/]=nav",
        );

        // Separator
        self.gam.draw_rectangle(
            self.content,
            Rectangle::new_with_style(
                Point::new(MARGIN_LEFT, 44),
                Point::new(self.screensize.x - MARGIN_RIGHT, 45),
                DrawStyle {
                    fill_color: Some(PixelColor::Dark),
                    stroke_color: None,
                    stroke_width: 0,
                },
            ),
        ).ok();

        // Content area
        let content_top = 48isize;
        let content_bottom = self.screensize.y - STATUS_BAR_HEIGHT;

        let mut y = content_top;
        let end_line = (buffer.viewport_top + buffer.viewport_lines).min(buffer.lines.len());

        for line_idx in buffer.viewport_top..end_line {
            if y + LINE_HEIGHT_REGULAR > content_bottom {
                break;
            }
            let line = &buffer.lines[line_idx];
            if !line.is_empty() {
                self.post_text(
                    MARGIN_LEFT, y,
                    self.screensize.x - MARGIN_LEFT * 2, LINE_HEIGHT_REGULAR,
                    GlyphStyle::Regular,
                    line,
                );
            }

            // Cursor
            if line_idx == buffer.cursor.line {
                self.draw_cursor(MARGIN_LEFT, y, line, buffer.cursor.col, LINE_HEIGHT_REGULAR, GlyphStyle::Regular);
            }

            y += LINE_HEIGHT_REGULAR;
        }

        // Word count in status
        let status = format!("Words: {}", buffer.word_count());
        let bar_top = self.screensize.y - STATUS_BAR_HEIGHT;
        self.gam.draw_rectangle(
            self.content,
            Rectangle::new_with_style(
                Point::new(0, bar_top),
                Point::new(self.screensize.x, bar_top + 1),
                DrawStyle {
                    fill_color: Some(PixelColor::Dark),
                    stroke_color: None,
                    stroke_width: 0,
                },
            ),
        ).ok();
        self.post_text(
            MARGIN_LEFT, bar_top + 4,
            self.screensize.x - MARGIN_LEFT * 2, STATUS_BAR_HEIGHT - 4,
            GlyphStyle::Small,
            &status,
        );

        self.finish();
    }

    // ---- Journal Search ----

    pub fn draw_journal_search(&self, query: &str, results: &[(String, String)], cursor: usize) {
        self.clear();

        self.post_text(
            MARGIN_LEFT, 8,
            self.screensize.x - MARGIN_LEFT * 2, 24,
            GlyphStyle::Bold,
            "SEARCH JOURNAL",
        );

        // Search input
        let input_display = format!("Query: {}|", query);
        self.post_text(
            MARGIN_LEFT, 40,
            self.screensize.x - MARGIN_LEFT * 2, 20,
            GlyphStyle::Regular,
            &input_display,
        );

        // Results
        let results_top = 70;
        let line_height = 28;

        if results.is_empty() && !query.is_empty() {
            self.post_text(
                20, results_top as isize,
                self.screensize.x - 40, 20,
                GlyphStyle::Small,
                "No matches found. Press ENTER to search.",
            );
        } else if results.is_empty() {
            self.post_text(
                20, results_top as isize,
                self.screensize.x - 40, 20,
                GlyphStyle::Small,
                "Type query, then ENTER to search",
            );
        } else {
            for (i, (date, line)) in results.iter().enumerate() {
                let y = results_top as isize + (i as isize) * line_height;
                if y + line_height > self.screensize.y - 40 {
                    break;
                }

                // Highlight selected result
                if i == cursor {
                    // Draw selection background
                    self.gam.draw_rectangle(
                        self.content,
                        Rectangle::new_with_style(
                            Point::new(8, y - 2),
                            Point::new(self.screensize.x - 8, y + line_height - 4),
                            DrawStyle {
                                fill_color: Some(PixelColor::Dark),
                                stroke_color: None,
                                stroke_width: 0,
                            },
                        ),
                    ).ok();
                }

                let truncated = format!("{}: {}", date, truncate_str(line, 28));

                // Create inverted text for selected item
                if i == cursor {
                    let mut tv = TextView::new(
                        self.content,
                        TextBounds::BoundingBox(Rectangle::new_coords(
                            12, y,
                            self.screensize.x - 12, y + line_height - 2,
                        ))
                    );
                    tv.style = GlyphStyle::Small;
                    tv.clear_area = false;
                    tv.invert = true;
                    use std::fmt::Write;
                    write!(tv.text, "{}", truncated).ok();
                    self.gam.post_textview(&mut tv).ok();
                } else {
                    self.post_text(
                        12, y,
                        self.screensize.x - 24, line_height - 2,
                        GlyphStyle::Small,
                        &truncated,
                    );
                }
            }
        }

        // Help text
        let help_text = if results.is_empty() {
            "F4=back  ENTER=search"
        } else {
            "↑↓=select  ENTER=go  F4=back"
        };
        self.post_text(
            MARGIN_LEFT, self.screensize.y - 36,
            self.screensize.x - MARGIN_LEFT * 2, 28,
            GlyphStyle::Small,
            help_text,
        );

        self.finish();
    }

    // ---- Typewriter ----

    pub fn draw_typewriter(&self, buffer: &TextBuffer) {
        self.clear();

        let content_top = 4isize;
        let content_bottom = self.screensize.y - STATUS_BAR_HEIGHT;

        let mut y = content_top;
        let end_line = (buffer.viewport_top + buffer.viewport_lines).min(buffer.lines.len());

        for line_idx in buffer.viewport_top..end_line {
            if y + LINE_HEIGHT_REGULAR > content_bottom {
                break;
            }
            let line = &buffer.lines[line_idx];
            if !line.is_empty() {
                self.post_text(
                    MARGIN_LEFT, y,
                    self.screensize.x - MARGIN_LEFT * 2, LINE_HEIGHT_REGULAR,
                    GlyphStyle::Regular,
                    line,
                );
            }

            // Cursor at end of last line
            if line_idx == buffer.cursor.line {
                self.draw_cursor(MARGIN_LEFT, y, line, buffer.cursor.col, LINE_HEIGHT_REGULAR, GlyphStyle::Regular);
            }

            y += LINE_HEIGHT_REGULAR;
        }

        // Status bar with word count
        let bar_top = self.screensize.y - STATUS_BAR_HEIGHT;
        self.gam.draw_rectangle(
            self.content,
            Rectangle::new_with_style(
                Point::new(0, bar_top),
                Point::new(self.screensize.x, bar_top + 1),
                DrawStyle {
                    fill_color: Some(PixelColor::Dark),
                    stroke_color: None,
                    stroke_width: 0,
                },
            ),
        ).ok();

        let status = format!("TYPEWRITER  W:{}  F1=menu F4=done", buffer.word_count());
        self.post_text(
            MARGIN_LEFT, bar_top + 4,
            self.screensize.x - MARGIN_LEFT * 2, STATUS_BAR_HEIGHT - 4,
            GlyphStyle::Small,
            &status,
        );

        self.finish();
    }

    // ---- Typewriter Done ----

    pub fn draw_typewriter_done(&self, words: usize, chars: usize, lines: usize) {
        self.clear();

        self.post_text(
            MARGIN_LEFT, 20,
            self.screensize.x - MARGIN_LEFT * 2, 30,
            GlyphStyle::Bold,
            "SESSION COMPLETE",
        );

        let stats = [
            format!("Words: {}", format_number(words)),
            format!("Characters: {}", format_number(chars)),
            format!("Lines: {}", format_number(lines)),
        ];

        let stats_top = 70;
        let line_height = 28;

        for (i, stat) in stats.iter().enumerate() {
            let y = stats_top + (i as isize) * line_height;
            self.post_text(
                30, y,
                self.screensize.x - 60, line_height - 2,
                GlyphStyle::Regular,
                stat,
            );
        }

        self.post_text(
            MARGIN_LEFT, self.screensize.y - 50,
            self.screensize.x - MARGIN_LEFT * 2, 40,
            GlyphStyle::Small,
            "s=save as doc  F4=discard",
        );

        self.finish();
    }
}
