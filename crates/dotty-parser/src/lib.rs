use crate::interpreter::Interpreter;
use crate::parser::Parser;

mod interpreter;
mod lexer;
mod parser;

pub use interpreter::{Action, Chmod, Clone, Context, Copy, Link, Step};

#[derive(Debug)]
pub struct DottyConfig {
    pub steps: Vec<Step>,
}

impl DottyConfig {
    pub fn parse(source: &str) -> anyhow::Result<Self> {
        Self::parse_with_context(source, Context::current()?)
    }

    pub fn parse_with_context(source: &str, context: Context) -> anyhow::Result<Self> {
        let ast = Parser::new(source).parse()?;
        let steps = Interpreter::from_context(context).run(ast)?;
        Ok(Self { steps })
    }

    pub fn links(&self) -> Vec<&Link> {
        self.steps
            .iter()
            .filter_map(|s| match s {
                Step::Link(l) => Some(l),
                _ => None,
            })
            .collect()
    }

    pub fn actions(&self) -> Vec<&Action> {
        self.steps
            .iter()
            .filter_map(|s| match s {
                Step::Action(a) => Some(a),
                _ => None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_parses_1() {
        let config = r#"
            link "dotfiles/.bashrc" to "~/.bashrc"
            link "dotfiles/.vimrc" to "~/.vimrc"

            do "echo 'Hello, world!'"
        "#;

        let parsed = DottyConfig::parse_with_context(
            config,
            Context {
                os: "linux".into(),
                hostname: "test-machine".into(),
                profile: None,
            },
        );
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        assert_eq!(parsed.links().len(), 2);
        assert_eq!(parsed.actions().len(), 1);
    }

    #[test]
    fn it_parses_2() {
        let config = r#"
            if os is "linux" {
                link "dotfiles/.bashrc" to "~/.bashrc"
            }

            if os is "macos" {
                link "dotfiles/.zshrc" to "~/.zshrc"
            }

            do "echo 'Hello, world!'"
        "#;

        let parsed = DottyConfig::parse_with_context(
            config,
            Context {
                os: "linux".into(),
                hostname: "test-machine".into(),
                profile: None,
            },
        );
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        assert_eq!(parsed.links().len(), 1);
        assert_eq!(parsed.actions().len(), 1);
    }

    #[test]
    fn it_parses_3() {
        let config = r#"
            # Declare dir prefixes
            common = "hosts/common"
            mac    = "hosts/mac"
            work   = "hosts/work"

            link "$common/config/git" to "~/.config/git"

            if os is "macos" {
              link "$mac/zshrc" to "~/.zshrc"
            }

            if os is not "linux" {
              link "$common/config/karabiner" to "~/.config/karabiner"
            }

            if hostname is "work-laptop" {
              link "$work/vimrc" to "~/.vimrc"
            }

            # Only install Zap if it's not already on the system
            if not test "zap" {
              do "zsh <(curl -s https://raw.githubusercontent.com/zap-zsh/zap/master/install.zsh) --branch release-v1"
            }

            # Run with a specific shell
            do "fish" "fisher install jorgebucaran/autopair.fish"

            # Profile-based config
            if profile is "work" {
              link "$work/ssh-config" to "~/.ssh/config"
            } else {
              link "$common/ssh-config" to "~/.ssh/config"
            }
        "#;

        let ctx = Context {
            os: "windows".into(),
            hostname: "work-laptop".into(),
            profile: Some("work".into()),
        };
        let parsed = DottyConfig::parse_with_context(config, ctx);
        assert!(parsed.is_ok(), "{}", parsed.unwrap_err());
        let parsed = parsed.unwrap();
        assert_eq!(parsed.links().len(), 4);
        assert_eq!(parsed.actions().len(), 2);
    }
}
