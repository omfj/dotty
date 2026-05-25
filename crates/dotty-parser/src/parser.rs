use std::mem;

use crate::lexer::{self, Lexer, Token, TokenStream};

const DEFAULT_SHELL: &str = "/bin/sh";

pub const OS: &str = "os";
pub const HOSTNAME: &str = "hostname";
pub const PROFILE: &str = "profile";
const SPECIAL_VARIABLES: &[&str] = &[OS, HOSTNAME, PROFILE];

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    String(String),
}

/// Represents a node in the AST
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Link {
        source: String,
        destination: String,
    },
    Do {
        command: String,
        shell: String,
    },
    If {
        condition: Box<Node>,
        true_branch: Vec<Node>,
        false_branch: Vec<Node>,
    },
    Is {
        left: Box<Node>,
        right: Box<Node>,
    },
    Or {
        left: Box<Node>,
        right: Box<Node>,
    },
    And {
        left: Box<Node>,
        right: Box<Node>,
    },
    Assign {
        variable: String,
        value: Value,
    },
    Env(String),
    Not(Box<Node>),
    Exists(String),
    Test(String),
    Print(Box<Node>),
    Variable(String),
    Literal(Value),
}

pub(crate) struct Parser<TS: TokenStream> {
    token_stream: TS,
    current_token: Option<lexer::Token>,
    current_line: usize,
}

impl<'a> Parser<Lexer<'a>> {
    pub fn new(source: &'a str) -> Self {
        let mut lexer = Lexer::new(source);
        let (current_token, current_line) = match lexer.next_token() {
            Some((t, l)) => (Some(t), l),
            None => (None, 1),
        };
        Self {
            token_stream: lexer,
            current_token,
            current_line,
        }
    }
}

impl<TS: TokenStream> Parser<TS> {
    #[cfg(test)]
    pub fn from_stream(mut stream: TS) -> Self {
        let (current_token, current_line) = match stream.next_token() {
            Some((t, l)) => (Some(t), l),
            None => (None, 0),
        };
        Self {
            token_stream: stream,
            current_token,
            current_line,
        }
    }

    pub fn parse(&mut self) -> anyhow::Result<Vec<Node>> {
        let mut nodes = Vec::new();

        while let Some(token) = &self.current_token {
            match token {
                Token::Link => self.handle_link(&mut nodes)?,
                Token::Do => self.handle_do(&mut nodes)?,
                Token::If => self.handle_if(&mut nodes)?,
                Token::Print => self.handle_print(&mut nodes)?,
                Token::Identifier(_) => self.handle_assignment(&mut nodes)?,
                _ => {
                    return Err(anyhow::anyhow!(
                        "line {}: unexpected token {:?}",
                        self.current_line,
                        token
                    ));
                }
            }
        }

        Ok(nodes)
    }

    // Similar to `Self::parse`, but it consumes until it finds a closing brace
    pub fn parse_block(&mut self) -> anyhow::Result<Vec<Node>> {
        self.expect_token(lexer::Token::LeftBrace)?;
        let mut nodes = Vec::new();

        while let Some(token) = &self.current_token {
            if *token == lexer::Token::RightBrace {
                self.consume();
                break;
            }

            match token {
                Token::Identifier(_) => self.handle_assignment(&mut nodes)?,
                Token::Link => self.handle_link(&mut nodes)?,
                Token::Do => self.handle_do(&mut nodes)?,
                Token::If => self.handle_if(&mut nodes)?,
                Token::Print => self.handle_print(&mut nodes)?,
                _ => {
                    return Err(anyhow::anyhow!(
                        "line {}: unexpected token in block {:?}",
                        self.current_line,
                        token
                    ));
                }
            }
        }

        Ok(nodes)
    }

    // Advances to the next token
    fn consume(&mut self) {
        match self.token_stream.next_token() {
            Some((t, l)) => {
                self.current_token = Some(t);
                self.current_line = l;
            }
            None => {
                self.current_token = None;
            }
        }
    }

    fn expect_token(&mut self, expected: lexer::Token) -> anyhow::Result<()> {
        if let Some(token) = &self.current_token {
            if mem::discriminant(token) == mem::discriminant(&expected) {
                self.consume();
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "line {}: expected {:?}, found {:?}",
                    self.current_line,
                    expected,
                    token
                ))
            }
        } else {
            Err(anyhow::anyhow!(
                "line {}: expected {:?}, found EOF",
                self.current_line,
                expected
            ))
        }
    }

    fn parse_condition(&mut self) -> anyhow::Result<Node> {
        let left = self.parse_comparison()?;
        match self.current_token.clone() {
            Some(Token::Or) => {
                self.consume();
                let right = self.parse_condition()?;
                Ok(Node::Or {
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
            Some(Token::And) => {
                self.consume();
                let right = self.parse_condition()?;
                Ok(Node::And {
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
            _ => Ok(left),
        }
    }

    fn parse_comparison(&mut self) -> anyhow::Result<Node> {
        if let Some(Token::Not) = &self.current_token {
            self.consume();
            let inner = self.parse_atom()?;
            return Ok(Node::Not(Box::new(inner)));
        }

        let left = self.parse_atom()?;
        match self.current_token.clone() {
            Some(Token::Is) => {
                self.consume();
                let right = self.parse_atom()?;
                Ok(Node::Is {
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
            Some(Token::IsNot) => {
                self.consume();
                let right = self.parse_atom()?;
                Ok(Node::Not(Box::new(Node::Is {
                    left: Box::new(left),
                    right: Box::new(right),
                })))
            }
            _ => Ok(left),
        }
    }

    fn parse_atom(&mut self) -> anyhow::Result<Node> {
        if let Some(token) = &self.current_token {
            match token {
                Token::Env => {
                    self.consume();
                    let variable = self.expect_string()?;
                    Ok(Node::Env(variable))
                }
                Token::Test => {
                    self.consume();
                    let command = self.expect_string()?;
                    Ok(Node::Test(command))
                }
                Token::Exists => {
                    self.consume();
                    let path = self.expect_string()?;
                    Ok(Node::Exists(path))
                }
                Token::String(value) => {
                    let value = value.clone();
                    self.consume();
                    Ok(Node::Literal(Value::String(value)))
                }
                Token::Identifier(name) => {
                    let name = name.clone();
                    self.consume();
                    Ok(Node::Variable(name))
                }
                _ => Err(anyhow::anyhow!(
                    "line {}: expected a value, found {:?}",
                    self.current_line,
                    token
                )),
            }
        } else {
            Err(anyhow::anyhow!(
                "line {}: expected a value, found EOF",
                self.current_line
            ))
        }
    }

    fn expect_string(&mut self) -> anyhow::Result<String> {
        if let Some(Token::String(value)) = &self.current_token {
            let value = value.clone();
            self.consume();
            Ok(value)
        } else {
            Err(anyhow::anyhow!(
                "line {}: expected a string, found {:?}",
                self.current_line,
                self.current_token
            ))
        }
    }

    fn optionally_expect_string(&mut self) -> Option<String> {
        self.expect_string().ok()
    }

    fn handle_link(&mut self, nodes: &mut Vec<Node>) -> anyhow::Result<()> {
        let line = self.current_line;
        self.consume();

        let source = self.parse_atom()?;
        let source = if let Node::Literal(Value::String(source_str)) = source {
            source_str
        } else {
            return Err(anyhow::anyhow!(
                "line {}: link source must be a string, found {:?}",
                line,
                source
            ));
        };

        self.expect_token(lexer::Token::To)?;
        let destination = self.parse_atom()?;
        let destination = if let Node::Literal(Value::String(dest_str)) = destination {
            dest_str
        } else {
            return Err(anyhow::anyhow!(
                "line {}: link destination must be a string, found {:?}",
                line,
                destination
            ));
        };

        nodes.push(Node::Link {
            source,
            destination,
        });

        Ok(())
    }

    fn handle_do(&mut self, nodes: &mut Vec<Node>) -> anyhow::Result<()> {
        self.consume();

        let arg1 = self.expect_string()?;
        let arg2 = self.optionally_expect_string();

        if let Some(command) = arg2 {
            nodes.push(Node::Do {
                command,
                shell: arg1,
            });
        } else {
            nodes.push(Node::Do {
                command: arg1,
                shell: DEFAULT_SHELL.to_string(),
            });
        }

        Ok(())
    }

    fn handle_print(&mut self, nodes: &mut Vec<Node>) -> anyhow::Result<()> {
        self.consume();
        let expr = self.parse_atom()?;
        nodes.push(Node::Print(Box::new(expr)));
        Ok(())
    }

    fn handle_if(&mut self, nodes: &mut Vec<Node>) -> anyhow::Result<()> {
        self.consume();

        let condition = self.parse_condition()?;
        let true_branch = self.parse_block()?;
        let false_branch = if let Some(lexer::Token::Else) = &self.current_token {
            self.consume();
            self.parse_block()?
        } else {
            Vec::new()
        };

        nodes.push(Node::If {
            condition: Box::new(condition),
            true_branch,
            false_branch,
        });

        Ok(())
    }

    fn handle_assignment(&mut self, nodes: &mut Vec<Node>) -> anyhow::Result<()> {
        let line = self.current_line;
        if let Some(Token::Identifier(variable)) = &self.current_token {
            let variable = variable.clone();
            if SPECIAL_VARIABLES.contains(&variable.as_str()) {
                return Err(anyhow::anyhow!(
                    "line {}: cannot assign to special variable '{}'",
                    line,
                    variable
                ));
            }
            self.consume();
            self.expect_token(lexer::Token::Assign)?;
            let value = self.expect_string()?;
            nodes.push(Node::Assign {
                variable,
                value: Value::String(value),
            });
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "line {}: expected an identifier, found {:?}",
                line,
                self.current_token
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::TokenList;

    use super::*;

    #[test]
    fn it_parses_link() {
        let token_stream: TokenList = vec![
            Token::Link,
            Token::String("dotfiles/.bashrc".to_string()),
            Token::To,
            Token::String("~/.bashrc".to_string()),
        ]
        .into();

        let nodes = Parser::from_stream(token_stream).parse().unwrap();

        assert_eq!(
            nodes,
            vec![Node::Link {
                source: "dotfiles/.bashrc".to_string(),
                destination: "~/.bashrc".to_string(),
            }]
        );
    }

    #[test]
    fn it_parses_do() {
        let token_stream: TokenList =
            vec![Token::Do, Token::String("echo 'Hello, world!'".to_string())].into();

        let nodes = Parser::from_stream(token_stream).parse().unwrap();

        assert_eq!(
            nodes,
            vec![Node::Do {
                command: "echo 'Hello, world!'".to_string(),
                shell: DEFAULT_SHELL.to_string(),
            }]
        );
    }

    #[test]
    fn it_parses_if() {
        let token_stream: TokenList = vec![
            Token::If,
            Token::Identifier("os".to_string()),
            Token::Is,
            Token::String("linux".to_string()),
            Token::LeftBrace,
            Token::Do,
            Token::String("echo 'Linux!'".to_string()),
            Token::RightBrace,
        ]
        .into();

        let nodes = Parser::from_stream(token_stream).parse().unwrap();

        assert_eq!(
            nodes,
            vec![Node::If {
                condition: Box::new(Node::Is {
                    left: Box::new(Node::Variable("os".to_string())),
                    right: Box::new(Node::Literal(Value::String("linux".to_string()))),
                }),
                true_branch: vec![Node::Do {
                    command: "echo 'Linux!'".to_string(),
                    shell: DEFAULT_SHELL.to_string(),
                }],
                false_branch: Vec::new(),
            }]
        );
    }

    #[test]
    fn it_parses_env() {
        let token_stream: TokenList = vec![
            Token::If,
            Token::Env,
            Token::String("SHELL".to_string()),
            Token::Is,
            Token::String("/bin/zsh".to_string()),
            Token::LeftBrace,
            Token::RightBrace,
        ]
        .into();

        let nodes = Parser::from_stream(token_stream).parse().unwrap();

        assert_eq!(
            nodes,
            vec![Node::If {
                condition: Box::new(Node::Is {
                    left: Box::new(Node::Env("SHELL".to_string())),
                    right: Box::new(Node::Literal(Value::String("/bin/zsh".to_string()))),
                }),
                true_branch: Vec::new(),
                false_branch: Vec::new(),
            }]
        )
    }

    #[test]
    fn it_parses_exists() {
        let token_stream: TokenList = vec![
            Token::If,
            Token::Exists,
            Token::String("dotfiles/.vimrc".to_string()),
            Token::LeftBrace,
            Token::RightBrace,
        ]
        .into();

        let nodes = Parser::from_stream(token_stream).parse().unwrap();

        assert_eq!(
            nodes,
            vec![Node::If {
                condition: Box::new(Node::Exists("dotfiles/.vimrc".to_string())),
                true_branch: Vec::new(),
                false_branch: Vec::new(),
            }]
        );
    }

    #[test]
    fn it_parses_is_not() {
        let token_stream: TokenList = vec![
            Token::If,
            Token::Env,
            Token::String("SHELL".to_string()),
            Token::IsNot,
            Token::String("/bin/zsh".to_string()),
            Token::LeftBrace,
            Token::Do,
            Token::String("echo 'Not using zsh!'".to_string()),
            Token::RightBrace,
        ]
        .into();

        let nodes = Parser::from_stream(token_stream).parse().unwrap();

        assert_eq!(
            nodes,
            vec![Node::If {
                condition: Box::new(Node::Not(Box::new(Node::Is {
                    left: Box::new(Node::Env("SHELL".to_string())),
                    right: Box::new(Node::Literal(Value::String("/bin/zsh".to_string()))),
                }))),
                true_branch: vec![Node::Do {
                    command: "echo 'Not using zsh!'".to_string(),
                    shell: DEFAULT_SHELL.to_string(),
                }],
                false_branch: Vec::new(),
            }]
        );
    }

    #[test]
    fn it_parses_assignments() {
        let token_stream: TokenList = vec![
            Token::Identifier("common".to_string()),
            Token::Assign,
            Token::String("hosts/common".to_string()),
            Token::Identifier("mac".to_string()),
            Token::Assign,
            Token::String("hosts/mac".to_string()),
        ]
        .into();

        let nodes = Parser::from_stream(token_stream).parse().unwrap();

        assert_eq!(
            nodes,
            vec![
                Node::Assign {
                    variable: "common".to_string(),
                    value: Value::String("hosts/common".to_string()),
                },
                Node::Assign {
                    variable: "mac".to_string(),
                    value: Value::String("hosts/mac".to_string()),
                },
            ]
        );
    }

    #[test]
    fn it_parses_if_with_or() {
        let token_stream: TokenList = vec![
            Token::If,
            Token::Identifier("os".to_string()),
            Token::Is,
            Token::String("linux".to_string()),
            Token::Or,
            Token::Identifier("os".to_string()),
            Token::Is,
            Token::String("macos".to_string()),
            Token::LeftBrace,
            Token::RightBrace,
        ]
        .into();

        let nodes = Parser::from_stream(token_stream).parse().unwrap();

        assert_eq!(
            nodes,
            vec![Node::If {
                condition: Box::new(Node::Or {
                    left: Box::new(Node::Is {
                        left: Box::new(Node::Variable("os".to_string())),
                        right: Box::new(Node::Literal(Value::String("linux".to_string()))),
                    }),
                    right: Box::new(Node::Is {
                        left: Box::new(Node::Variable("os".to_string())),
                        right: Box::new(Node::Literal(Value::String("macos".to_string()))),
                    }),
                }),
                true_branch: Vec::new(),
                false_branch: Vec::new(),
            }]
        );
    }
}
