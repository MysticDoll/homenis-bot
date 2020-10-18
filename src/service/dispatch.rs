use regex::Regex;
use std::collections::HashMap;

pub enum DispatchState {
    Complete(String),
    NotFound,
    Ignore,
}

pub struct DispatchService {
    jobs: HashMap<String, Box<dyn Fn(&str) -> Result<DispatchState, String>>>,
}

impl DispatchService {
    pub fn new() -> Self {
        Self {
            jobs: HashMap::new(),
        }
    }

    pub fn parse_and_exec(&self, msg: &str) -> Result<DispatchState, String> {
        let re = Regex::new(r"!(?P<command>\s+(?<body>.+))").unwrap();
        if let Some(caps) = re.captures(msg) {
            let command = caps
                .name("command")
                .ok_or("failed to get command")?
                .as_str();
            let body = caps.name("body").ok_or("failed to get body")?.as_str();
            if let Some(f) = self.jobs.get(command) {
                f(&body)
            } else {
                Ok(DispatchState::NotFound)
            }
        } else {
            Ok(DispatchState::Ignore)
        }
    }
}
