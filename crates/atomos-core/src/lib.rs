#![forbid(unsafe_code)]

pub mod audio;
pub mod governor;
pub mod turn;
pub mod voice;

/// No network, credentials, or external tools in this prototype.
pub trait Inference {
    fn respond(&mut self, text: &str) -> String;
}

#[derive(Default)]
pub struct MockInference {
    pub calls: usize,
}

impl Inference for MockInference {
    fn respond(&mut self, text: &str) -> String {
        self.calls += 1;
        format!("Mock response: {text}")
    }
}

/// Exact, explicit commands only. Unknown or ambiguous input is not executed.
pub fn resolve_local(text: &str) -> Option<&'static str> {
    match text.trim().to_ascii_lowercase().as_str() {
        "what is your name?" | "what's your name?" | "what is your name" | "what's your name" => Some("I am AtomOS."),
        "help" => Some("Commands: help, what is your name?, /quit."),
        _ => None,
    }
}

pub fn respond(provider: &mut impl Inference, text: &str) -> String {
    match resolve_local(text) {
        Some(answer) => answer.to_owned(),
        None => provider.respond(text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_commands_never_invoke_provider() {
        let mut provider = MockInference::default();
        assert_eq!(respond(&mut provider, " WHAT IS YOUR NAME? "), "I am AtomOS.");
        assert_eq!(provider.calls, 0);
        assert!(respond(&mut provider, "explain entropy").starts_with("Mock response:"));
        assert_eq!(provider.calls, 1);
    }

    #[test]
    fn ambiguous_command_is_not_locally_executed() {
        assert_eq!(resolve_local("help and delete my files"), None);
    }
}
