use serde::{Deserialize, Serialize};

/// Represents a prompt argument that can be passed to customize the prompt
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptArgument {
    /// The name of the argument
    pub name: String,
    /// A description of what the argument is used for
    pub description: String,
    /// Whether this argument is required
    pub required: bool,
}

/// A prompt that can be used to generate text from a model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    /// The name of the prompt
    pub name: String,
    /// A description of what the prompt does
    pub description: String,
    /// The arguments that can be passed to customize the prompt
    pub arguments: Vec<PromptArgument>,
}

impl Prompt {
    /// Create a new prompt with the given name, description and arguments
    pub fn new<N, D>(name: N, description: D, arguments: Vec<PromptArgument>) -> Self
    where
        N: Into<String>,
        D: Into<String>,
    {
        Prompt {
            name: name.into(),
            description: description.into(),
            arguments,
        }
    }
}
