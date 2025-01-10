use mcp_core::prompt::{Prompt, PromptArgument, PromptTemplate};
use std::fs;
use std::path::Path;

pub fn create_unit_test_prompt() -> Prompt {
    let prompt_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/developer/prompts/unit_test.json");

    let prompt_str = fs::read_to_string(&prompt_path).expect("Failed to read prompt template file");

    let template: PromptTemplate =
        serde_json::from_str(&prompt_str).expect("Failed to parse prompt template");

    let arguments = template
        .arguments
        .into_iter()
        .map(|arg| PromptArgument {
            name: arg.name.into(),
            description: arg.description.into(),
            required: arg.required,
        })
        .collect();

    Prompt::new(&template.id, &template.template, arguments)
}
