use anyhow::Result;
use std::collections::HashSet;

use goose::config::PermissionManager;
use goose::message::{Message, MessageContent, ToolRequest};
use goose::model::ModelConfig;
use goose::providers::base::ProviderUsage;
use goose::providers::create;
use goose::providers::errors::ProviderError;
use mcp_core::tool::Tool;

use goose::permission::permission_judge::{check_tool_permissions, PermissionCheckResult};

use serde::{Deserialize, Serialize};


// The tool request IDs of the tools that are approved, need approval, or are denied
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolApprovals {
    pub approved: Vec<String>,
    pub needs_approval: Vec<String>,
    pub denied: Vec<String>,
}

impl ToolApprovals {
    pub fn from_permission_check_result(permission_check_result: PermissionCheckResult) -> Self {
        Self {
            approved: permission_check_result.approved.iter().map(|t| t.id.clone()).collect(),
            needs_approval: permission_check_result.needs_approval.iter().map(|t| t.id.clone()).collect(),
            denied: permission_check_result.denied.iter().map(|t| t.id.clone()).collect(),
        }
    }
    
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    message: Message,
    usage: ProviderUsage,
    tool_approvals: Option<ToolApprovals>,
}

impl CompletionResponse {
    pub fn new(message: Message, usage: ProviderUsage) -> Self {
        Self {
            message,
            usage,
            tool_approvals: None,
        }
    }
}

/// Public API for the Goose LLM completion function
pub async fn completion(
    provider: &str,
    model_config: ModelConfig,
    system_preamble: &str,
    messages: &[Message],
    tools: &[Tool],
    check_tool_approval: bool,
) -> Result<CompletionResponse, ProviderError> {
    let provider = create(provider, model_config).unwrap();
    let system_prompt = construct_system_prompt(system_preamble, tools);

    let (response, usage) = provider.complete(&system_prompt, messages, tools).await?;
    let mut result = CompletionResponse::new(response.clone(), usage.clone());

    if check_tool_approval {
        // Check if the tool annotations are present
        let (tools_with_readonly_annotation, tools_without_annotation) =
            categorize_tools_by_annotation(tools);

        // Collect all tool requests from the response
        let tool_requests: Vec<ToolRequest> = response
            .content
            .iter()
            .filter_map(|content| {
                if let MessageContent::ToolRequest(req) = content {
                    Some(req.clone())
                } else {
                    None
                }
            })
            .collect();

        // Check for tool permissions using "smart_approve" mode
        let mut permission_manager = PermissionManager::default();
        let (permission_check_result, _) = check_tool_permissions(
            &tool_requests,
            "smart_approve",
            tools_with_readonly_annotation.clone(),
            tools_without_annotation.clone(),
            &mut permission_manager,
            provider.clone(),
        )
        .await;

        let tool_approvals = ToolApprovals::from_permission_check_result(permission_check_result);
        result.tool_approvals = Some( tool_approvals );
    }

    Ok(result)
}

/// Categorize tools based on their annotations
/// Returns:
/// - read_only_tools: Tools with read-only annotations
/// - non_read_tools: Tools without read-only annotations
fn categorize_tools_by_annotation(tools: &[Tool]) -> (HashSet<String>, HashSet<String>) {
    tools
        .iter()
        .fold((HashSet::new(), HashSet::new()), |mut acc, tool| {
            match &tool.annotations {
                Some(annotations) if annotations.read_only_hint => {
                    acc.0.insert(tool.name.clone());
                }
                _ => {
                    acc.1.insert(tool.name.clone());
                }
            }
            acc
        })
}

fn get_parameter_names(tool: &Tool) -> Vec<String> {
    tool.input_schema
        .get("properties")
        .and_then(|props| props.as_object())
        .map(|props| props.keys().cloned().collect())
        .unwrap_or_default()
}

fn construct_system_prompt(system_preamble: &str, tools: &[Tool]) -> String {
    let mut system_prompt = system_preamble.to_string();
    if !tools.is_empty() {
        system_prompt.push_str("\n\n");
        system_prompt.push_str("Tools available:\n");
        for tool in tools {
            system_prompt.push_str(&format!(
                "## {}\nDescription: {}\nParameters: {:?}\n",
                tool.name,
                tool.description,
                get_parameter_names(tool)
            ));
        }
    } else {
        system_prompt.push_str("\n\n");
        system_prompt.push_str("No tools available.\n");
    }
    system_prompt
}
