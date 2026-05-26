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

impl From<Link> for Step {
    fn from(link: Link) -> Self {
        Step::Link(link)
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

impl From<Action> for Step {
    fn from(action: Action) -> Self {
        Step::Action(action)
    }
}

#[derive(Debug, Clone)]
pub struct Clone {
    pub url: String,
    pub destination: String,
}

#[derive(Debug, Clone)]
pub struct Copy {
    pub source: String,
    pub destination: String,
}

#[derive(Debug, Clone)]
pub struct Chmod {
    pub path: String,
    pub mode: String,
}

#[derive(Debug)]
pub enum Step {
    Link(Link),
    Action(Action),
    CreateDir(String),
    Clone(Clone),
    Copy(Copy),
    Chmod(Chmod),
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

    pub fn with_profile(mut self, profile: String) -> Self {
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
    environment: Environment,
    created_dirs: std::collections::HashSet<String>,
}

impl Interpreter {
    #[allow(dead_code)]
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self::from_context(Context::current()?))
    }

    pub fn from_context(context: Context) -> Self {
        let mut environment = Environment::new();
        environment.set_variable(
            parser::OS.to_string(),
            parser::Value::String(context.os.clone()),
        );
        environment.set_variable(
            parser::HOSTNAME.to_string(),
            parser::Value::String(context.hostname.clone()),
        );
        environment.set_variable(
            parser::PROFILE.to_string(),
            parser::Value::String(context.profile.clone().unwrap_or_default()),
        );
        Self {
            environment,
            created_dirs: std::collections::HashSet::new(),
        }
    }

    fn emit_create_dir(&mut self, path: &str, steps: &mut Vec<Step>) {
        if let Some(parent) = std::path::Path::new(path).parent()
            && !parent.as_os_str().is_empty()
            && !parent.exists()
        {
            let dir = parent.to_string_lossy().into_owned();
            if self.created_dirs.insert(dir.clone()) {
                steps.push(Step::CreateDir(dir));
            }
        }
    }

    pub fn run(&mut self, nodes: Vec<parser::Node>) -> anyhow::Result<Vec<Step>> {
        let mut steps: Vec<Step> = Vec::new();

        for node in nodes {
            match node {
                parser::Node::Link {
                    source,
                    destination,
                } => {
                    let destination = self.interpolate(&destination)?;
                    self.emit_create_dir(&destination, &mut steps);
                    steps.push(Step::Link(Link {
                        source: self.interpolate(&source)?,
                        destination,
                    }));
                }
                parser::Node::Clone { url, destination } => {
                    let destination = self.interpolate(&destination)?;
                    self.emit_create_dir(&destination, &mut steps);
                    steps.push(Step::Clone(Clone {
                        url: self.interpolate(&url)?,
                        destination,
                    }));
                }
                parser::Node::Copy {
                    source,
                    destination,
                } => {
                    let destination = self.interpolate(&destination)?;
                    self.emit_create_dir(&destination, &mut steps);
                    steps.push(Step::Copy(Copy {
                        source: self.interpolate(&source)?,
                        destination,
                    }));
                }
                parser::Node::Chmod { path, mode } => {
                    steps.push(Step::Chmod(Chmod {
                        path: self.interpolate(&path)?,
                        mode,
                    }));
                }
                parser::Node::Do { command, shell } => {
                    steps.push(Step::Action(Action {
                        command: self.interpolate(&command)?,
                        shell,
                    }));
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
                    steps.extend(result);
                }
                parser::Node::Print(expr) => {
                    let parser::Value::String(s) = self.evaluate_expression(*expr)?;
                    println!("{}", s);
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

        Ok(steps)
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
            parser::Node::Variable(name) => match self.environment.get_variable(&name) {
                Some(value) => Ok(value.clone()),
                None => Err(anyhow::anyhow!("Undefined variable: {}", name)),
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
