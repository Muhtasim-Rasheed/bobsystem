#[derive(Debug)]
pub enum LexError {
    UnterminatedString { line: usize },
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LexError::UnterminatedString { line } => {
                write!(f, "line {line}: unterminated string literal")
            }
        }
    }
}

impl std::error::Error for LexError {}

#[derive(Debug, Clone)]
pub struct Token {
    pub val: String,
    pub line: usize,
}

pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    let mut tokens = Vec::new();
    let mut chars = src.chars().peekable();
    let mut current = String::new();
    let mut line = 1usize;

    fn flush(current: &mut String, tokens: &mut Vec<Token>, line: usize) {
        if !current.is_empty() {
            tokens.push(Token {
                val: std::mem::take(current),
                line,
            });
        }
    }

    while let Some(&ch) = chars.peek() {
        match ch {
            '\n' => {
                flush(&mut current, &mut tokens, line);
                line += 1;
                chars.next();
            }
            c if c.is_whitespace() => {
                flush(&mut current, &mut tokens, line);
                chars.next();
            }
            '/' => {
                let mut lookahead = chars.clone();
                lookahead.next();
                if lookahead.peek() == Some(&'/') {
                    flush(&mut current, &mut tokens, line);
                    while let Some(&c) = chars.peek() {
                        if c == '\n' {
                            break;
                        }
                        chars.next();
                    }
                } else {
                    current.push(ch);
                    chars.next();
                }
            }
            '"' => {
                flush(&mut current, &mut tokens, line);
                let mut s = String::from("\"");
                chars.next(); // consume opening quote
                loop {
                    match chars.next() {
                        None => return Err(LexError::UnterminatedString { line }),
                        Some('\n') => return Err(LexError::UnterminatedString { line }),
                        Some('"') => {
                            s.push('"');
                            break;
                        }
                        Some('\\') => match chars.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some('\\') => s.push('\\'),
                            Some('"') => s.push('"'),
                            Some(other) => s.push(other),
                            None => return Err(LexError::UnterminatedString { line }),
                        },
                        Some(c) => s.push(c),
                    }
                }
                tokens.push(Token { val: s, line });
            }
            _ => {
                current.push(ch);
                chars.next();
            }
        }
    }
    flush(&mut current, &mut tokens, line);

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    trait VecTokenExt {
        fn into_strings(self) -> Vec<String>;
    }

    impl VecTokenExt for Vec<Token> {
        fn into_strings(self) -> Vec<String> {
            self.into_iter()
                .map(|Token { val, line }| format!("{line}: {val}"))
                .collect()
        }
    }

    #[test]
    fn basic_tokens() {
        assert_eq!(
            lex("func main\n    \"foo bar\" puts // prints foo bar\nend")
                .unwrap()
                .into_strings(),
            vec!["1: func", "1: main", "2: \"foo bar\"", "2: puts", "3: end"]
        );
    }

    #[test]
    fn string_with_extra_internal_whitespace_is_preserved_exactly() {
        assert_eq!(
            lex(r#""foo  bar" puts"#).unwrap().into_strings(),
            vec!["1: \"foo  bar\"", "1: puts"]
        );
    }

    #[test]
    fn slashes_inside_a_string_are_not_a_comment() {
        assert_eq!(
            lex(r#""http://example.com" puts"#).unwrap().into_strings(),
            vec!["1: \"http://example.com\"", "1: puts"]
        );
    }

    #[test]
    fn comment_after_a_string_is_still_stripped() {
        assert_eq!(
            lex(r#""foo" puts // this // has slashes too"#)
                .unwrap()
                .into_strings(),
            vec!["1: \"foo\"", "1: puts"]
        );
    }

    #[test]
    fn unterminated_string_is_an_error() {
        assert!(matches!(
            lex("\"never closed"),
            Err(LexError::UnterminatedString { line: 1 })
        ));
    }

    #[test]
    fn escaped_quote_inside_string() {
        assert_eq!(
            lex(r#""say \"hi\"" puts"#).unwrap().into_strings(),
            vec!["1: \"say \"hi\"\"", "1: puts"]
        );
    }
}
