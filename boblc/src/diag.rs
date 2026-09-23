use std::rc::Rc;

use colored::Colorize;

use crate::source::{SourceFile, Span};

#[derive(Debug)]
pub enum Severity {
    Error,
    Warning,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "{}", "Error".red().underline()),
            Self::Warning => write!(f, "{}", "Warning".yellow().underline()),
        }
    }
}

#[derive(Debug)]
pub struct Label {
    pub message: Option<String>,
    pub span: Span,
    pub important: bool,
}

#[derive(Debug)]
pub enum Help {
    Text(String),
}

#[derive(Debug)]
pub struct Diag {
    pub source_file: Rc<SourceFile>,

    pub severity: Severity,
    pub message: String,
    pub labels: Vec<Label>,
    pub helps: Vec<Help>,
}

impl Diag {
    pub fn new(source_file: &Rc<SourceFile>, message: impl ToString) -> Self {
        Self {
            source_file: Rc::clone(source_file),
            severity: Severity::Error,
            message: message.to_string(),
            labels: vec![],
            helps: vec![],
        }
    }

    pub fn warning(source_file: &Rc<SourceFile>, message: impl ToString) -> Self {
        Self {
            source_file: Rc::clone(source_file),
            severity: Severity::Warning,
            message: message.to_string(),
            labels: vec![],
            helps: vec![],
        }
    }

    pub fn with_label(mut self, span: Span, important: bool) -> Self {
        self.labels.push(Label {
            message: None,
            span,
            important,
        });
        self
    }

    pub fn with_message_label(
        mut self,
        message: impl ToString,
        span: Span,
        important: bool,
    ) -> Self {
        self.labels.push(Label {
            message: Some(message.to_string()),
            span,
            important,
        });
        self
    }

    pub fn with_text_help(mut self, message: impl ToString) -> Self {
        self.helps.push(Help::Text(message.to_string()));
        self
    }

    pub fn render(&self, out: &mut String) {
        macro_rules! outln {
            ($fmt:expr $(, $($arg:tt)*)?) => {{
                use std::fmt::Write;
                writeln!(out, $fmt $(, $($arg)*)?).unwrap()
            }};
        }

        let primary = self
            .labels
            .iter()
            .find(|l| l.important)
            .or_else(|| self.labels.first());

        if let Some(primary) = primary {
            let pos = self.source_file.offset_to_position(primary.span.lo);
            outln!(
                "[{}:{}] {}: {}",
                self.source_file.name,
                pos,
                self.severity,
                self.message
            );
        } else {
            outln!(
                "[{}] {}: {}",
                self.source_file.name,
                self.severity,
                self.message
            );
        }

        for (i, label) in self.labels.iter().enumerate() {
            let pos = self.source_file.offset_to_position(label.span.lo);
            let line_text = self.source_file.line_text(pos.line);

            let span_chars = self.source_file.content[label.span.lo..label.span.hi]
                .chars()
                .count()
                .max(1);
            let remaining_on_line = line_text.chars().count().saturating_sub(pos.col - 1);
            let underline_len = span_chars.min(remaining_on_line.max(1));

            let marker = if label.important { '^' } else { '-' };
            let marker_disp = {
                let str = marker.to_string().repeat(underline_len);
                if label.important {
                    str.red()
                } else {
                    str.blue()
                }
            };
            let label_message = if label.important {
                label.message.as_deref().unwrap_or("").red()
            } else {
                label.message.as_deref().unwrap_or("").blue()
            };
            let gutter_width = pos.line.to_string().len().max(4);

            if i == 0 {
                outln!("{}", format!("{:>gutter_width$} |", "").blue());
            }
            outln!(
                "{} {}",
                format!("{:>gutter_width$} |", pos.line).blue(),
                expand_tabs(line_text)
            );
            outln!(
                "{} {}{} {}",
                format!("{:>gutter_width$} |", "").blue(),
                " ".repeat(visual_column(line_text, pos.col)),
                marker_disp,
                label_message
            );
        }

        for help in &self.helps {
            match help {
                Help::Text(help_text) => {
                    outln!("{} {help_text}", format!("Help:").green().underline());
                }
            }
        }
    }
}

const TAB_WIDTH: usize = 4;

fn expand_tabs(s: &str) -> String {
    let mut out = String::new();
    let mut col = 0;
    for c in s.chars() {
        if c == '\t' {
            let spaces = TAB_WIDTH - (col % TAB_WIDTH);
            out.push_str(&" ".repeat(spaces));
            col += spaces;
        } else {
            out.push(c);
            col += 1;
        }
    }
    out
}

fn visual_column(line_text: &str, char_col: usize) -> usize {
    let mut visual = 0;
    for c in line_text.chars().take(char_col - 1) {
        if c == '\t' {
            visual += TAB_WIDTH - (visual % TAB_WIDTH);
        } else {
            visual += 1;
        }
    }
    visual
}
