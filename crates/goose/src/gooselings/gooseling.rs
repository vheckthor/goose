use crate::agents::extension::ExtensionConfig;
use serde::{Deserialize, Serialize};

fn default_version() -> String {
    "1.0.0".to_string()
}

/// A Gooseling represents a personalized, user-generated agent configuration that defines
/// specific behaviors and capabilities within the Goose system.
///
/// # Fields
///
/// ## Required Fields
/// * `version` - Semantic version of the Gooseling file format (defaults to "1.0.0")
/// * `title` - Short, descriptive name of the Gooseling
/// * `description` - Detailed description explaining the Gooseling's purpose and functionality
/// * `Instructions` - Instructions that defines the Gooseling's behavior
///
/// ## Optional Fields
/// * `extensions` - List of extension configurations required by the Gooseling
/// * `goosehints` - Additional goosehints to be merged with existing .goosehints configuration
/// * `context` - Supplementary context information for the Gooseling
/// * `activities` - Activity labels that appear when loading the Gooseling
/// * `settings` - Configuration settings including model preferences
/// * `author` - Information about the Gooseling's creator and metadata
///
/// # Example
///
/// ```
/// use your_crate::Gooseling;
///
/// let gooseling = Gooseling {
///     version: "1.0.0".to_string(),
///     title: "Example Agent".to_string(),
///     description: "An example Gooseling configuration".to_string(),
///     instructions: "Act as a helpful assistant".to_string(),
///     extensions: None,
///     goosehints: None,
///     context: None,
///     activities: None,
///     settings: None,
///     author: None,
/// };
/// ```
#[derive(Serialize, Deserialize, Debug)]
pub struct Gooseling {
    // Required fields
    #[serde(default = "default_version")]
    pub version: String, // version of the file format, sem ver

    pub title: String, // short title of the gooseling

    pub description: String, // a longer description of the gooseling

    pub instructions: String, // the instructions for the model

    // Optional fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<ExtensionConfig>>, // a list of extensions to enable

    #[serde(skip_serializing_if = "Option::is_none")]
    pub goosehints: Option<String>, // any additional goosehints to merge with existing .goosehints

    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<String>>, // any additional context

    #[serde(skip_serializing_if = "Option::is_none")]
    pub activities: Option<Vec<String>>, // the activity pills that show up when loading the

    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Settings>, // any additional settings information

    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Author>, // any additional author information
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>, // settings/model; optionally provided
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Author {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>, // creator/contact information of the gooseling

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>, // any additional metadata for the author
}
