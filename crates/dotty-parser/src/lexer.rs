const IF_KEYWORD: &str = "if";
const ELSE_KEYWORD: &str = "else";
const LINK_KEYWORD: &str = "link";
const ENV_KEYWORD: &str = "env";
const EXISTS_KEYWORD: &str = "exists";
const DO_KEYWORD: &str = "do";
const TEST_KEYWORD: &str = "test";
const PRINT_KEYWORD: &str = "print";
const CLONE_KEYWORD: &str = "clone";
const COPY_KEYWORD: &str = "copy";
const CHMOD_KEYWORD: &str = "chmod";

const TO_OPERATOR: &str = "to";
const NOT_OPERATOR: &str = "not";
const IS_OPERATOR: &str = "is";
const OR_OPERATOR: &str = "or";
const AND_OPERATOR: &str = "and";
const ASSIGN_OPERATOR: &str = "=";

const LEFT_BRACE: char = '{';
const RIGHT_BRACE: char = '}';

const COMMENT_CHAR: char = '#';

const QUOTE_CHARS: &[char] = &['"', '\''];

const EOL_CHARS: &[char] = &['\n', '\r'];

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Token {
    Link,
    To,
    Is,
    IsNot,
    Or,
    And,
    LeftBrace,
    RightBrace,
    Env,
    If,
    Else,
    ElseIf,
    Exists,
    Do,
    Assign,
    Not,
    Test,
    Print,
    Clone,
    Copy,
    Chmod,

    String(String),
    Identifier(String),
}

impl TryFrom<&str> for Token {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            LINK_KEYWORD => Ok(Token::Link),
            TO_OPERATOR => Ok(Token::To),
            IS_OPERATOR => Ok(Token::Is),
            OR_OPERATOR => Ok(Token::Or),
            AND_OPERATOR => Ok(Token::And),
            ENV_KEYWORD => Ok(Token::Env),
            IF_KEYWORD => Ok(Token::If),
            ELSE_KEYWORD => Ok(Token::Else),
            EXISTS_KEYWORD => Ok(Token::Exists),
            DO_KEYWORD => Ok(Token::Do),
            ASSIGN_OPERATOR => Ok(Token::Assign),
            NOT_OPERATOR => Ok(Token::Not),
            TEST_KEYWORD => Ok(Token::Test),
            PRINT_KEYWORD => Ok(Token::Print),
            CLONE_KEYWORD => Ok(Token::Clone),
            COPY_KEYWORD => Ok(Token::Copy),
            CHMOD_KEYWORD => Ok(Token::Chmod),
            _ => Err(anyhow::anyhow!("Unknown token: {}", value)),
        }
    }
}

pub(crate) trait TokenStream {
    fn next_token(&mut self) -> Option<(Token, usize)>;
}

// Only a simple helper for testing purposes now.

#[cfg(test)]
pub(crate) struct TokenList(std::collections::VecDeque<Token>);

#[cfg(test)]
impl From<Vec<Token>> for TokenList {
    fn from(tokens: Vec<Token>) -> Self {
        Self(tokens.into())
    }
}

#[cfg(test)]
impl TokenStream for TokenList {
    fn next_token(&mut self) -> Option<(Token, usize)> {
        self.0.pop_front().map(|t| (t, 0))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Lexer<'a> {
    source: &'a str,
    position: usize,
    line: usize,
}

impl TokenStream for Lexer<'_> {
    fn next_token(&mut self) -> Option<(Token, usize)> {
        self.skip_whitespace();

        if self.is_eof() {
            return None;
        }

        let token_line = self.line;

        if self.source[self.position..].starts_with(LEFT_BRACE) {
            self.position += 1;
            return Some((Token::LeftBrace, token_line));
        }

        if self.source[self.position..].starts_with(RIGHT_BRACE) {
            self.position += 1;
            return Some((Token::RightBrace, token_line));
        }

        if self.source[self.position..].starts_with(COMMENT_CHAR) {
            let end_comment = self.source[self.position..]
                .find(|c: char| EOL_CHARS.contains(&c))
                .map(|i| i + self.position)
                .unwrap_or(self.source.len());
            self.position = end_comment;
            return self.next_token();
        }

        for &quote in QUOTE_CHARS {
            if self.source[self.position..].starts_with(quote) {
                let end_quote = self.source[self.position + 1..]
                    .find(quote)
                    .map(|i| i + self.position + 1)?;
                let string = self.source[self.position + 1..end_quote].to_string();
                self.position = end_quote + 1;
                return Some((Token::String(string), token_line));
            }
        }

        let word_end = self.source[self.position..]
            .find(|c: char| c.is_whitespace() || c == LEFT_BRACE || c == RIGHT_BRACE)
            .map(|i| i + self.position)
            .unwrap_or(self.source.len());
        let word = &self.source[self.position..word_end];

        if let Ok(token) = Token::try_from(word) {
            self.position = word_end;

            if token == Token::Else && self.peek_next_token() == Some(Token::If) {
                self.position += IF_KEYWORD.len() + 1; // Skip "else if" and the following whitespace
                return Some((Token::ElseIf, token_line));
            }

            if token == Token::Is && self.peek_next_token() == Some(Token::Not) {
                self.position += NOT_OPERATOR.len() + 1; // Skip "is not" and the following whitespace
                return Some((Token::IsNot, token_line));
            }

            return Some((token, token_line));
        }

        self.position = word_end;
        Some((Token::Identifier(word.to_string()), token_line))
    }
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            position: 0,
            line: 1,
        }
    }

    fn peek_next_token(&mut self) -> Option<Token> {
        let saved_pos = self.position;
        let saved_line = self.line;
        let token = self.next_token().map(|(t, _)| t);
        self.position = saved_pos;
        self.line = saved_line;
        token
    }

    fn skip_whitespace(&mut self) {
        while !self.is_eof() && self.is_at_whitespace() {
            if self.source[self.position..].starts_with('\n') {
                self.line += 1;
            }
            self.position += 1;
        }
    }

    fn is_eof(&self) -> bool {
        self.position >= self.source.len()
    }

    fn is_at_whitespace(&self) -> bool {
        self.source[self.position..].starts_with(char::is_whitespace)
    }
}

#[cfg(test)]
impl Lexer<'_> {
    fn next_tok(&mut self) -> Option<Token> {
        self.next_token().map(|t| t.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_lexes_1() {
        let str = r#"link "dotfiles/.bashrc" to "~/.bashrc""#;

        let mut lexer = Lexer::new(str);
        assert_eq!(lexer.next_tok(), Some(Token::Link));
        assert_eq!(
            lexer.next_tok(),
            Some(Token::String("dotfiles/.bashrc".to_string()))
        );
        assert_eq!(lexer.next_tok(), Some(Token::To));
        assert_eq!(
            lexer.next_tok(),
            Some(Token::String("~/.bashrc".to_string()))
        );
        assert_eq!(lexer.next_tok(), None);
    }

    #[test]
    fn it_lexes_2() {
        let str = r#"do "echo 'Hello, world!'" "#;

        let mut lexer = Lexer::new(str);
        assert_eq!(lexer.next_tok(), Some(Token::Do));
        assert_eq!(
            lexer.next_tok(),
            Some(Token::String("echo 'Hello, world!'".to_string()))
        );
        assert_eq!(lexer.next_tok(), None);
    }

    #[test]
    fn it_lexes_3() {
        let str = r#"if os is "linux" {
            do "echo 'Linux!'"
        } else if os is "windows" {
            do "echo 'Windows!'"
        } else {
            do "echo 'Other OS!'"
        }
        "#;

        let mut lexer = Lexer::new(str);
        assert_eq!(lexer.next_tok(), Some(Token::If));
        assert_eq!(lexer.next_tok(), Some(Token::Identifier("os".to_string())));
        assert_eq!(lexer.next_tok(), Some(Token::Is));
        assert_eq!(lexer.next_tok(), Some(Token::String("linux".to_string())));
        assert_eq!(lexer.next_tok(), Some(Token::LeftBrace));
        assert_eq!(lexer.next_tok(), Some(Token::Do));
        assert_eq!(
            lexer.next_tok(),
            Some(Token::String("echo 'Linux!'".to_string()))
        );
        assert_eq!(lexer.next_tok(), Some(Token::RightBrace));
        assert_eq!(lexer.next_tok(), Some(Token::ElseIf));
        assert_eq!(lexer.next_tok(), Some(Token::Identifier("os".to_string())));
        assert_eq!(lexer.next_tok(), Some(Token::Is));
        assert_eq!(lexer.next_tok(), Some(Token::String("windows".to_string())));
        assert_eq!(lexer.next_tok(), Some(Token::LeftBrace));
        assert_eq!(lexer.next_tok(), Some(Token::Do));
        assert_eq!(
            lexer.next_tok(),
            Some(Token::String("echo 'Windows!'".to_string()))
        );
        assert_eq!(lexer.next_tok(), Some(Token::RightBrace));
        assert_eq!(lexer.next_tok(), Some(Token::Else));
        assert_eq!(lexer.next_tok(), Some(Token::LeftBrace));
        assert_eq!(lexer.next_tok(), Some(Token::Do));
        assert_eq!(
            lexer.next_tok(),
            Some(Token::String("echo 'Other OS!'".to_string()))
        );
        assert_eq!(lexer.next_tok(), Some(Token::RightBrace));
    }

    #[test]
    fn it_lexes_4() {
        let str = r#"
            common = "hosts/common"
            mac    = "hosts/mac"
        "#;

        let mut lexer = Lexer::new(str);
        assert_eq!(
            lexer.next_tok(),
            Some(Token::Identifier("common".to_string()))
        );
        assert_eq!(lexer.next_tok(), Some(Token::Assign));
        assert_eq!(
            lexer.next_tok(),
            Some(Token::String("hosts/common".to_string()))
        );
        assert_eq!(lexer.next_tok(), Some(Token::Identifier("mac".to_string())));
        assert_eq!(lexer.next_tok(), Some(Token::Assign));
        assert_eq!(
            lexer.next_tok(),
            Some(Token::String("hosts/mac".to_string()))
        );
        assert_eq!(lexer.next_tok(), None);
    }

    #[test]
    fn it_lexes_5() {
        let str = r#"if not test "echo" {
                do "echo 'echo is not available!'"
            }
        "#;

        let mut lexer = Lexer::new(str);
        assert_eq!(lexer.next_tok(), Some(Token::If));
        assert_eq!(lexer.next_tok(), Some(Token::Not));
        assert_eq!(lexer.next_tok(), Some(Token::Test));
        assert_eq!(lexer.next_tok(), Some(Token::String("echo".to_string())));
        assert_eq!(lexer.next_tok(), Some(Token::LeftBrace));
        assert_eq!(lexer.next_tok(), Some(Token::Do));
        assert_eq!(
            lexer.next_tok(),
            Some(Token::String("echo 'echo is not available!'".to_string()))
        );
        assert_eq!(lexer.next_tok(), Some(Token::RightBrace));
    }

    #[test]
    fn it_lexes_6() {
        let str = r#"if hostname is not "my-work-book" {}"#;

        let mut lexer = Lexer::new(str);
        assert_eq!(lexer.next_tok(), Some(Token::If));
        assert_eq!(
            lexer.next_tok(),
            Some(Token::Identifier("hostname".to_string()))
        );
        assert_eq!(lexer.next_tok(), Some(Token::IsNot));
        assert_eq!(
            lexer.next_tok(),
            Some(Token::String("my-work-book".to_string()))
        );
        assert_eq!(lexer.next_tok(), Some(Token::LeftBrace));
        assert_eq!(lexer.next_tok(), Some(Token::RightBrace));
    }
}
