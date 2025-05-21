use anyhow::Result;
use console::style;
// No import from crate::recipes::print_recipe

use crate::recipes::search_recipe::retrieve_recipe_file;
use goose::recipe::{Recipe, RecipeParameter, RecipeParameterRequirement};
use minijinja::{Environment, Error, Template, UndefinedBehavior};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub const BUILT_IN_RECIPE_DIR_PARAM: &str = "recipe_dir";
pub const RECIPE_FILE_EXTENSIONS: &[&str] = &["yaml", "json"];

// `load_recipe_as_template` function removed

/// Loads and validates a recipe from a YAML or JSON file
///
/// # Arguments
///
/// * `recipe_name` - Name of the recipe to load
/// * `log`  - whether to log information about the recipe or not
/// * `params` - optional parameters to render the recipe with
///
/// # Returns
///
/// The parsed recipe struct if successful
///
/// # Errors
///
/// Returns an error if:
/// - The file doesn't exist or can't be read
/// - The YAML/JSON is invalid
/// - The required fields are missing
/// - Parameter validation fails
pub fn load_recipe(
    recipe_name: &str,
    log: bool,
    params: Option<Vec<(String, String)>>,
) -> Result<Recipe> {
    let (recipe_file_content, recipe_parent_dir) = retrieve_recipe_file(recipe_name)?;
    let recipe_parameters = validate_recipe_file_parameters(&recipe_file_content, &recipe_parent_dir)?;

    let (rendered_content, params_for_template_map) = if let Some(user_params) = params {
        let resolved_params = apply_values_to_parameters(&user_params, recipe_parameters, &recipe_parent_dir)?;
        (
            render_content_with_params(&recipe_file_content, &resolved_params)?,
            Some(resolved_params),
        )
    } else {
        // Attempt to apply defaults if no user params provided
        let default_params = apply_values_to_parameters(&[], recipe_parameters, &recipe_parent_dir)?;
        (
            render_content_with_params(&recipe_file_content, &default_params)?,
            Some(default_params),
        )
    };

    let recipe = parse_recipe_content(&rendered_content)?;

    if log {
        println!(
            "{} {}",
            style("Loading recipe:").green().bold(),
            style(&recipe.title).green()
        );
        println!("{} {}", style("Description:").dim(), &recipe.description);

        if let Some(pft_map) = params_for_template_map {
            if !pft_map.is_empty() {
                println!("{}", style("Parameters:").dim());
                for (key, value) in pft_map {
                    println!("{}: {}", key, value);
                }
            }
        }
        println!();
    }
    Ok(recipe)
}

// `explain_recipe_with_parameters` function removed

fn validate_recipe_file_parameters(recipe_file_content: &str, recipe_parent_dir: &PathBuf) -> Result<Vec<RecipeParameter>> {
    let recipe_from_recipe_file: Recipe = parse_recipe_content(recipe_file_content)?;
    validate_optional_parameters(&recipe_from_recipe_file)?;
    validate_parameters_in_template(recipe_from_recipe_file, recipe_file_content)
}

fn validate_parameters_in_template(
    recipe: Recipe,
    recipe_file_content: &str,
) -> Result<Vec<RecipeParameter>> {
    let template_variables = extract_template_variables(recipe_file_content)?;

    let param_keys: HashSet<String> = recipe
        .parameters
        .as_ref()
        .unwrap_or(&vec![])
        .iter()
        .map(|p| p.key.clone())
        .collect();

    let missing_keys = template_variables
        .difference(&param_keys)
        .filter(|&key| key != BUILT_IN_RECIPE_DIR_PARAM) // Exclude built-in if auto-injected
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let extra_keys = param_keys
        .difference(&template_variables)
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    if missing_keys.is_empty() && extra_keys.is_empty() {
        return Ok(recipe.parameters.unwrap_or_default());
    }

    let mut message = String::new();
    if !missing_keys.is_empty() {
        message.push_str(&format!(
            "Missing definitions for parameters in the recipe file: {}.",
            missing_keys.join(", ")
        ));
    }
    if !extra_keys.is_empty() {
        if !message.is_empty() {
            message.push('\\n');
        }
        message.push_str(&format!(
            "Unnecessary parameter definitions: {}.",
            extra_keys.join(", ")
        ));
    }
    Err(anyhow::anyhow!("{}", message.trim()))
}

fn validate_optional_parameters(recipe: &Recipe) -> Result<()> {
    let optional_params_without_default_values: Vec<String> = recipe
        .parameters
        .as_ref()
        .unwrap_or(&vec![])
        .iter()
        .filter(|p| {
            matches!(p.requirement, RecipeParameterRequirement::Optional) && p.default.is_none()
        })
        .map(|p| p.key.clone())
        .collect();

    if optional_params_without_default_values.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Optional parameters missing default values in the recipe: {}. Please provide defaults.", optional_params_without_default_values.join(", ")))
    }
}

fn parse_recipe_content(content: &str) -> Result<Recipe> {
    if serde_json::from_str::<JsonValue>(content).is_ok() {
        Ok(serde_json::from_str(content)?)
    } else if serde_yaml::from_str::<YamlValue>(content).is_ok() {
        Ok(serde_yaml::from_str(content)?)
    } else {
        Err(anyhow::anyhow!(
            "Unsupported file format for recipe file. Expected .yaml or .json"
        ))
    }
}

fn extract_template_variables(template_str: &str) -> Result<HashSet<String>> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    let template = env
        .template_from_str(template_str)
        .map_err(|e: Error| anyhow::anyhow!("Invalid template syntax: {}", e.to_string()))?;
    Ok(template.undeclared_variables(true))
}

fn apply_values_to_parameters(
    user_params: &[(String, String)],
    recipe_parameters: Vec<RecipeParameter>,
    recipe_parent_dir: &PathBuf,
) -> Result<HashMap<String, String>> {
    let mut param_map: HashMap<String, String> = user_params.iter().cloned().collect();

    if let Some(dir_str) = recipe_parent_dir.to_str() {
        param_map.entry(BUILT_IN_RECIPE_DIR_PARAM.to_string()).or_insert_with(|| dir_str.to_string());
    } else {
        return Err(anyhow::anyhow!("Invalid UTF-8 in recipe_dir path"));
    }

    let mut missing_required_params: Vec<String> = Vec::new();

    for param_def in recipe_parameters {
        if !param_map.contains_key(&param_def.key) {
            match (&param_def.default, &param_def.requirement) {
                (Some(default_val), _) => {
                    param_map.insert(param_def.key.clone(), default_val.clone());
                }
                (None, RecipeParameterRequirement::Required) | (None, RecipeParameterRequirement::UserPrompt) => {
                    // Treat UserPrompt as Required if no value given, as per pre-e968e0022c logic
                    missing_required_params.push(param_def.key.clone());
                }
                (None, RecipeParameterRequirement::Optional) => {
                    // Optional without default, not provided: OK
                }
            }
        }
    }

    if missing_required_params.is_empty() {
        Ok(param_map)
    } else {
        let formatted_missing = missing_required_params
            .iter()
            .map(|key| format!("--params {}=<value>", key)) // Suggestion format
            .collect::<Vec<_>>()
            .join(" ");
        Err(anyhow::anyhow!(
            "Missing required parameters that were not provided and have no defaults: {}",
            formatted_missing
        ))
    }
}

fn render_content_with_params(content: &str, params: &HashMap<String, String>) -> Result<String> {
    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    let template: Template = env.template_from_str(content)
        .map_err(|e: Error| anyhow::anyhow!("Failed to render recipe, please check if the recipe has proper syntax for variables (e.g., {{ variable_name }}): {}", e.to_string()))?;
    template.render(params).map_err(|e: Error| {
        anyhow::anyhow!(
            "Failed to render the recipe with provided parameters ({}). Please check if all required template variables are supplied.",
            e.to_string()
        )
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use goose::recipe::{RecipeParameterInputType, RecipeParameterRequirement};
    use tempfile::TempDir;
    use super::*;

    fn setup_recipe_file(instructions_and_parameters: &str) -> (TempDir, PathBuf) {
        let recipe_content = format!(
            r#"{{"version": "1.0.0", "title": "Test Recipe", "description": "A test recipe", {}}}"#,
            instructions_and_parameters
        );
        let temp_dir = tempfile::tempdir().unwrap();
        let recipe_path: PathBuf = temp_dir.path().join("test_recipe.json");
        std::fs::write(&recipe_path, recipe_content).unwrap();
        (temp_dir, recipe_path)
    }

    #[test]
    fn test_render_content_with_params() {
        let content = "Hello {{ name }}!";
        let mut params = HashMap::new();
        params.insert("name".to_string(), "World".to_string());
        assert_eq!(render_content_with_params(content, &params).unwrap(), "Hello World!");

        let content_missing = "Hello {{ missing }}!";
        let err = render_content_with_params(content_missing, &HashMap::new()).unwrap_err();
        assert!(err.to_string().contains("Failed to render the recipe"));
    }

    #[test]
    fn test_load_recipe_success() {
        let instructions_and_parameters = r#"
            "instructions": "Test instructions with {{ my_name }}",
            "parameters": [
                {"key": "my_name", "input_type": "string", "requirement": "required", "description": "A test parameter"}
            ]"#;
        let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);
        let params = vec![("my_name".to_string(), "value".to_string())];
        let recipe = load_recipe(recipe_path.to_str().unwrap(), false, Some(params)).unwrap();
        assert_eq!(recipe.title, "Test Recipe");
        assert_eq!(recipe.instructions.unwrap(), "Test instructions with value");
    }

    #[test]
    fn test_load_recipe_success_variable_in_prompt() { // "prompt" key in recipe content
        let instructions_and_parameters = r#"
            "instructions": "Test instructions",
            "prompt": "My prompt {{ my_name }}",
            "parameters": [
                {"key": "my_name", "input_type": "string", "requirement": "required", "description": "A test parameter"}
            ]"#;
        let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);
        let params = vec![("my_name".to_string(), "value".to_string())];
        let recipe = load_recipe(recipe_path.to_str().unwrap(), false, Some(params)).unwrap();
        assert_eq!(recipe.prompt.unwrap(), "My prompt value");
    }

    #[test]
    fn test_load_recipe_wrong_parameters_in_recipe_file() {
        let instructions_and_parameters = r#"
            "instructions": "Test instructions with {{ expected_param1 }} {{ expected_param2 }}",
            "parameters": [
                {"key": "wrong_param_key", "input_type": "string", "requirement": "required", "description": "A test parameter"}
            ]"#;
        let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);
        let err = load_recipe(recipe_path.to_str().unwrap(), false, None).unwrap_err();
        assert!(err.to_string().contains("Unnecessary parameter definitions: wrong_param_key"));
        assert!(err.to_string().contains("Missing definitions for parameters in the recipe file: expected_param1, expected_param2"));
    }

    #[test]
    fn test_load_recipe_with_default_values_in_recipe_file() {
        let instructions_and_parameters = r#"
            "instructions": "Test with {{ param_with_default }} and {{ param_without_default }}",
            "parameters": [
                {"key": "param_with_default", "input_type": "string", "requirement": "optional", "default": "default_value", "description": "Test"},
                {"key": "param_without_default", "input_type": "string", "requirement": "required", "description": "Test"}
            ]"#;
        let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);
        let params = vec![("param_without_default".to_string(), "user_value".to_string())];
        let recipe = load_recipe(recipe_path.to_str().unwrap(), false, Some(params)).unwrap();
        assert_eq!(recipe.instructions.unwrap(), "Test with default_value and user_value");
    }

    #[test]
    fn test_load_recipe_optional_parameters_without_default_values_in_recipe_file() {
        let instructions_and_parameters = r#"
            "instructions": "Test instructions with {{ optional_param }}",
            "parameters": [
                {"key": "optional_param", "input_type": "string", "requirement": "optional", "description": "A test parameter"}
            ]"#;
        let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);
        let err = load_recipe(recipe_path.to_str().unwrap(), false, None).unwrap_err();
        // This test might change behavior depending on how strict `apply_values_to_parameters` is with rendering missing optionals
        // If it attempts to render, it will fail if `optional_param` is not in params.
        // If `apply_values_to_parameters` doesn't add it to the map, and minijinja is strict, render will fail.
        // The old `validate_optional_parameters` would pass this. The error comes from rendering.
        assert!(err.to_string().contains("Failed to render the recipe"));
    }
    
    #[test]
    fn test_load_recipe_optional_param_missing_no_default_renders_empty_if_template_allows() {
        // This test assumes minijinja with StrictUndefined behavior would normally error.
        // However, if the parameter ISN'T in the rendering map, and the template uses it, it will error.
        // If the parameter has a default (even empty string), it would be in map.
        // Let's test providing an empty default for an optional param.
         let instructions_and_parameters = r#"
            "instructions": "Test instructions with '{{ optional_param }}'",
            "parameters": [
                {"key": "optional_param", "input_type": "string", "requirement": "optional", "default": "", "description": "A test parameter"}
            ]"#;
        let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);
        let recipe = load_recipe(recipe_path.to_str().unwrap(), false, None).unwrap(); // No params, should use default
        assert_eq!(recipe.instructions.unwrap(), "Test instructions with ''");
    }


    #[test]
    fn test_load_recipe_wrong_input_type_in_recipe_file() {
        let instructions_and_parameters = r#"
            "instructions": "Test instructions",
            "parameters": [
                {"key": "param", "input_type": "some_invalid_type", "requirement": "required", "description": "A test parameter"}
            ]"#;
        let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);
        let params = vec![("param".to_string(), "value".to_string())];
        let err = load_recipe(recipe_path.to_str().unwrap(), false, Some(params)).unwrap_err();
        // Error comes from parsing Recipe struct due to invalid enum variant for RecipeParameterInputType
        assert!(err.to_string().contains("unknown variant `some_invalid_type`"));
    }

    #[test]
    fn test_load_recipe_success_without_parameters() {
        let instructions_and_parameters = r#""instructions": "Test instructions no params""#;
        let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);
        let recipe = load_recipe(recipe_path.to_str().unwrap(), false, None).unwrap();
        assert_eq!(recipe.instructions.unwrap(), "Test instructions no params");
        assert!(recipe.parameters.as_ref().map_or(true, |p| p.is_empty()));
    }
}
