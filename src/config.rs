#[derive(Debug, Clone)]
pub struct Link {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone)]
pub struct Action {
    pub name: String,
    pub command: String,
    pub shell: String,
}

#[derive(Debug, Clone, Default)]
pub struct DottyConfig {
    pub links: Vec<Link>,
    pub actions: Vec<Action>,
    pub overwrite: bool,
    pub ask: bool,
    pub selected_profile: Option<String>,
}

impl DottyConfig {
    pub fn with_overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    pub fn with_ask(mut self, ask: bool) -> Self {
        self.ask = ask;
        self
    }

    pub fn with_profile(mut self, profile: Option<String>) -> Self {
        self.selected_profile = profile;
        self
    }
}
