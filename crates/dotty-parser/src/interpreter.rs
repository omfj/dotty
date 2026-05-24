use std::collections::HashMap;

use crate::parser;

#[derive(Debug, Clone)]
pub struct Link {
    pub source: String,
    pub destination: String,
}

impl TryFrom<parser::Node> for Link {
    type Error = anyhow::Error;

    fn try_from(value: parser::Node) -> Result<Self, Self::Error> {
        match value {
            parser::Node::Link {
                source,
                destination,
            } => Ok(Self {
                source,
                destination,
            }),
            _ => Err(anyhow::anyhow!("Expected a Link node")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Action {
    pub command: String,
    pub shell: String,
}

impl TryFrom<parser::Node> for Action {
    type Error = anyhow::Error;

    fn try_from(value: parser::Node) -> Result<Self, Self::Error> {
        match value {
            parser::Node::Do { command, shell } => Ok(Self { command, shell }),
            _ => Err(anyhow::anyhow!("Expected an Action node")),
        }
    }
}

pub struct InterpreterResult {
    pub links: Vec<Link>,
    pub actions: Vec<Action>,
}

pub struct Environment {
    variables: HashMap<String, parser::Value>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn set_variable(&mut self, name: String, value: parser::Value) {
        self.variables.insert(name, value);
    }

    pub fn get_variable(&self, name: &str) -> Option<&parser::Value> {
        self.variables.get(name)
    }
}

pub struct Context {
    pub os: String,
    pub hostname: String,
    pub profile: Option<String>,
}

impl Context {
    pub fn current() -> anyhow::Result<Self> {
        let os = Self::get_os();
        let hostname = Self::get_hostname()?;
        Ok(Self {
            os,
            hostname,
            profile: None,
        })
    }

    #[allow(dead_code)]
    fn with_profile(mut self, profile: String) -> Self {
        self.profile = Some(profile);
        self
    }

    fn get_os() -> String {
        std::env::consts::OS.to_string()
    }

    fn get_hostname() -> anyhow::Result<String> {
        let output = std::process::Command::new("hostname")
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to get hostname: {}", e))?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

pub struct Interpreter {
    context: Context,
    environment: Environment,
}

impl Interpreter {
    #[allow(dead_code)]
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self::from_context(Context::current()?))
    }

    pub fn from_context(context: Context) -> Self {
        let environment = Environment::new();
        Self {
            context,
            environment,
        }
    }

    pub fn run(&mut self, nodes: Vec<parser::Node>) -> anyhow::Result<InterpreterResult> {
        let mut links = Vec::new();
        let mut actions = Vec::new();

        for node in nodes {
            match node {
                parser::Node::Link { .. } => {
                    links.push(Link::try_from(node)?);
                }
                parser::Node::Do { .. } => {
                    actions.push(Action::try_from(node)?);
                }
                parser::Node::If {
                    condition,
                    true_branch,
                    false_branch,
                } => {
                    let result = if self.evaluate_condition(*condition)? {
                        self.run(true_branch)?
                    } else {
                        self.run(false_branch)?
                    };
                    links.extend(result.links);
                    actions.extend(result.actions);
                }
                parser::Node::Assign { variable, value } => {
                    self.environment.set_variable(variable, value);
                }
                node => {
                    return Err(anyhow::anyhow!(
                        "Unexpected node in statement position: {:?}",
                        node
                    ));
                }
            }
        }

        Ok(InterpreterResult { links, actions })
    }

    fn evaluate_condition(&self, node: parser::Node) -> anyhow::Result<bool> {
        match node {
            parser::Node::Is { left, right } => {
                Ok(self.evaluate_expression(*left)? == self.evaluate_expression(*right)?)
            }
            parser::Node::Not(inner) => Ok(!self.evaluate_condition(*inner)?),
            parser::Node::Or { left, right } => {
                Ok(self.evaluate_condition(*left)? || self.evaluate_condition(*right)?)
            }
            parser::Node::And { left, right } => {
                Ok(self.evaluate_condition(*left)? && self.evaluate_condition(*right)?)
            }
            parser::Node::Exists(path) => Ok(std::path::Path::new(&path).exists()),
            parser::Node::Test(command) => Ok(std::process::Command::new("which")
                .arg(&command)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)),
            node => Err(anyhow::anyhow!("Expected a condition, found: {:?}", node)),
        }
    }

    fn evaluate_expression(&self, node: parser::Node) -> anyhow::Result<parser::Value> {
        match node {
            parser::Node::Literal(parser::Value::String(s)) => {
                Ok(parser::Value::String(self.interpolate(&s)?))
            }
            parser::Node::Env(var) => {
                let value = std::env::var(&var)
                    .map_err(|_| anyhow::anyhow!("Environment variable '{}' not set", var))?;
                Ok(parser::Value::String(value))
            }
            parser::Node::Variable(name) => match name.as_str() {
                parser::OS => Ok(parser::Value::String(self.context.os.clone())),
                parser::HOSTNAME => Ok(parser::Value::String(self.context.hostname.clone())),
                parser::PROFILE => Ok(parser::Value::String(
                    self.context.profile.clone().unwrap_or_default(),
                )),
                _ => match self.environment.get_variable(&name) {
                    Some(value) => Ok(value.clone()),
                    None => Err(anyhow::anyhow!("Undefined variable: {}", name)),
                },
            },
            node => Err(anyhow::anyhow!("Expected a value, found: {:?}", node)),
        }
    }

    fn interpolate(&self, s: &str) -> anyhow::Result<String> {
        let mut result = String::new();
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' {
                let mut name = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        name.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                match self.environment.get_variable(&name) {
                    Some(parser::Value::String(v)) => result.push_str(v),
                    None => return Err(anyhow::anyhow!("Undefined variable: ${}", name)),
                }
            } else {
                result.push(c);
            }
        }

        Ok(result)
    }
}
