use mcp_core::prompt::{Prompt, PromptArgument};

pub fn create_unit_test_prompt() -> Prompt {
    Prompt::new(
        "unit_test",
        indoc::indoc! {r#"
            Generate or update unit tests for a given source code file.
            
            The test suite should:
            - Follow language-specific test naming conventions
            - Include all necessary imports and annotations
            - Thoroughly test the specified functionality
            - Ensure tests are passing before completion
            - Handle edge cases and error conditions
            - Use clear test names that reflect what is being tested
        "#},
        vec![
            PromptArgument {
                name: "source_code".into(),
                description: "The source code file content to be tested".into(),
                required: true,
            },
            PromptArgument {
                name: "language".into(),
                description: "The programming language of the source code".into(),
                required: true,
            },
            PromptArgument {
                name: "existing_tests".into(),
                description: "Any existing test code that needs to be updated".into(),
                required: false,
            },
        ],
    )
} 