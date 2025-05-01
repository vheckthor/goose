use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::agents::extension_manager::ExtensionManager;
use crate::model::ModelConfig;
use crate::providers::databricks::DatabricksProvider;
use mcp_client::client::{ClientCapabilities, ClientInfo, Error as ClientError, McpClientTrait};
use mcp_core::protocol::{CallToolResult, GetPromptResult, Implementation, InitializeResult, ListPromptsResult, ListResourcesResult, ListToolsResult, ReadResourceResult, ServerCapabilities, ToolsCapability};
// use mcp_core::protocol::METHOD_NOT_FOUND;
use mcp_core::tool::{Tool, ToolCall};
use mcp_core::{Content, ToolError};

/// Simple vector index for semantic search
struct VectorIndex {
    /// Map of keys to embeddings
    embeddings: HashMap<String, Vec<f32>>,

    /// Dimension of embeddings
    dimension: usize,
}

impl VectorIndex {
    /// Create a new vector index with the specified dimension
    fn new(dimension: usize) -> Self {
        Self {
            embeddings: HashMap::new(),
            dimension,
        }
    }

    /// Add a key-embedding pair to the index
    fn add(&mut self, key: &str, embedding: &[f32]) {
        assert_eq!(
            embedding.len(),
            self.dimension,
            "Embedding dimension mismatch"
        );
        self.embeddings.insert(key.to_string(), embedding.to_vec());
    }

    /// Search for the top k most similar embeddings
    fn search(&self, query: &[f32], k: usize) -> Vec<(String, f32)> {
        assert_eq!(query.len(), self.dimension, "Query dimension mismatch");

        // Calculate cosine similarity for each embedding
        let mut scores: Vec<(String, f32)> = self
            .embeddings
            .iter()
            .map(|(key, emb)| {
                let similarity = cosine_similarity(query, emb);
                (key.clone(), similarity)
            })
            .collect();

        // Sort by similarity (highest first)
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return top k results
        scores.truncate(k);
        scores
    }
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

/// ToolRouterExtension implements McpClientTrait and acts as a proxy for tool calls
#[derive(Clone)]
pub struct ToolRouterExtension {
    /// Reference to the ToolRouter instance
    router: Arc<Mutex<ToolRouter>>,
}

impl ToolRouterExtension {
    /// Create a new ToolRouterExtension with the given ToolRouter
    pub fn new(router: Arc<Mutex<ToolRouter>>) -> Self {
        Self { router }
    }
}

#[async_trait]
impl McpClientTrait for ToolRouterExtension {
    async fn initialize(
        &mut self,
        _info: ClientInfo,
        _capabilities: ClientCapabilities,
    ) -> Result<InitializeResult, ClientError> {
        // Return basic initialization result
        Ok(InitializeResult {
            protocol_version: "1.0".to_string(),
            server_info: Implementation {
                name: "tool_router".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("ToolRouter provides dynamic tool discovery and activation at runtime.".to_string()),
            capabilities: ServerCapabilities {
                prompts: None,
                resources: None,
                tools: Some(ToolsCapability {
                    list_changed: None,
                }),
            },
        })
    }

    async fn list_resources(
        &self,
        _next_cursor: Option<String>,
    ) -> Result<ListResourcesResult, ClientError> {
        // ToolRouter doesn't provide resources
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    async fn read_resource(&self, _uri: &str) -> Result<ReadResourceResult, ClientError> {
        // ToolRouter doesn't provide resources
        Err(ClientError::NotInitialized)
    }

    async fn list_tools(&self, _next_cursor: Option<String>) -> Result<ListToolsResult, ClientError> {
        // Return the tools provided by the ToolRouter
        let router = self.router.lock().await;
        Ok(ListToolsResult {
            tools: router.tools(),
            next_cursor: None,
        })
    }

    async fn call_tool(&self, name: &str, arguments: serde_json::Value) -> Result<CallToolResult, ClientError> {
        // Create a ToolCall and pass it to the ToolRouter's handle_call method
        // Add the toolrouter__ prefix if it's not already there
        let prefixed_name = if name.starts_with("toolrouter__") {
            name.to_string()
        } else {
            format!("toolrouter__{}", name)
        };
        
        let tool_call = ToolCall {
            name: prefixed_name,
            arguments,
        };
        
        let mut router = self.router.lock().await;
        let result = router.handle_call(tool_call).await
            .map_err(|_e| ClientError::NotInitialized)?;
            
        match result {
            Ok(contents) => Ok(CallToolResult {
                content: contents,
                is_error: None,
            }),
            Err(_tool_error) => Err(ClientError::NotInitialized),
        }
    }

    async fn list_prompts(
        &self,
        _next_cursor: Option<String>,
    ) -> Result<ListPromptsResult, ClientError> {
        // ToolRouter doesn't provide prompts
        Ok(ListPromptsResult {
            prompts: vec![],
        })
    }

    async fn get_prompt(
        &self,
        _name: &str,
        _arguments: serde_json::Value,
    ) -> Result<GetPromptResult, ClientError> {
        // ToolRouter doesn't provide prompts
        Err(ClientError::NotInitialized)
    }
}

/// ToolRouter provides dynamic tool discovery and activation at runtime.
/// It uses vector search to find relevant tools based on user intent.
pub struct ToolRouter {
    /// Vector index for semantic search of tools
    index: VectorIndex,

    /// Storage for all available tools - maps tool name to (extension_name, Tool)
    tools: HashMap<String, (String, Tool)>,

    /// Set of currently active tool names
    active_tools: HashSet<String>,

    /// Original clients for proxying tool calls - maps extension name to client
    #[allow(dead_code)]
    clients: HashMap<String, Arc<Mutex<Box<dyn McpClientTrait>>>>,
    
    /// Databricks provider for embeddings
    embedding_provider: Option<DatabricksProvider>,
}

impl ToolRouter {
    /// Create a new ToolRouter instance and index all available tools
    pub async fn new(extension_manager: &ExtensionManager) -> Result<Self> {
        tracing::debug!("ToolRouter: Creating new instance");
        // Try to initialize the Databricks provider for embeddings
        let embedding_provider = match ModelConfig::new("text-embedding-3-small".to_string()) {
            model => match DatabricksProvider::from_env(model) {
                Ok(provider) => {
                    tracing::debug!("ToolRouter: Successfully initialized Databricks provider for embeddings");
                    Some(provider)
                },
                Err(e) => {
                    tracing::warn!("ToolRouter: Failed to initialize Databricks provider for embeddings: {}", e);
                    None
                }
            }
        };
        
        let mut router = Self {
            // Use 384 dimensions for text-embedding-3-small
            index: VectorIndex::new(1536),
            tools: HashMap::new(),
            active_tools: HashSet::new(),
            clients: HashMap::new(),
            embedding_provider,
        };

        // Get all available tools from the extension manager
        tracing::warn!("ToolRouter: Fetching tools from extension manager");
        
        // Get list of all extensions
        let extensions = extension_manager.list_extensions().await?;
        
        // For each extension, get its tools and store them
        for extension_name in extensions {
            tracing::warn!("ToolRouter: Processing extension: {}", extension_name);
            
            // Get tools for this extension
            let extension_tools = extension_manager.get_prefixed_tools(Some(extension_name.clone())).await?;
            tracing::debug!("ToolRouter: Found {} tools for extension {}", extension_tools.len(), extension_name);
            
            // Index each tool
            for tool in extension_tools {
                tracing::debug!("ToolRouter: Indexing tool: {}", tool.name);
                router.add_tool(&extension_name, tool).await?;
            }
            
            // Store client reference for later use in proxying
            // Note: In a real implementation, we would need to get the client from the extension manager
            // This is a placeholder for now
            // router.clients.insert(extension_name, client_reference);
        }

        tracing::debug!("ToolRouter: Initialization complete with {} tools indexed", router.tools.len());
        Ok(router)
    }

    /// Add a tool to the router's index and storage
    async fn add_tool(&mut self, extension_name: &str, tool: Tool) -> Result<()> {
        let tool_key = tool.name.clone();

        // Create text to embed (name + description)
        let embedding_text = format!("{}: {}", tool.name, tool.description);

        // Generate embedding for the tool
        let embedding = self.embed_text(&embedding_text).await?;

        // Add to vector index and tool storage
        self.index.add(&tool_key, &embedding);
        self.tools.insert(tool_key, (extension_name.to_string(), tool));

        Ok(())
    }

    /// Generate embeddings for text using Databricks text-embedding-3-small if available,
    /// or fall back to a simple deterministic algorithm
    async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        tracing::debug!("ToolRouter: Generating embedding for text: {}", text);
        
        // Try to use Databricks provider if available
        if let Some(provider) = &self.embedding_provider {
            tracing::debug!("ToolRouter: Using Databricks provider for embeddings");
            
            // Create payload for the embedding request
            let payload = json!({
                "input": text,
            });
            
            // Use the provider's post method which handles auth
            match provider.post(payload).await {
                Ok(response) => {
                    // Extract embedding from response
                    if let Some(data) = response.get("data") {
                        if let Some(embeddings) = data.get(0) {
                            if let Some(embedding_array) = embeddings.get("embedding") {
                                if let Some(array) = embedding_array.as_array() {
                                    let embedding: Vec<f32> = array
                                        .iter()
                                        .filter_map(|v| v.as_f64().map(|f| f as f32))
                                        .collect();
                                    
                                    if embedding.len() == self.index.dimension {
                                        tracing::debug!("ToolRouter: Successfully got embedding from Databricks");
                                        return Ok(embedding);
                                    } else {
                                        tracing::warn!(
                                            "ToolRouter: Embedding dimension mismatch: got {}, expected {}",
                                            embedding.len(),
                                            self.index.dimension
                                        );
                                    }
                                }
                            }
                        }
                    }
                    
                    tracing::warn!("ToolRouter: Failed to parse embedding response: {:?}", response);
                }
                Err(e) => {
                    tracing::warn!("ToolRouter: Error getting embedding from Databricks: {}", e);
                }
            }
        }
        
        // Fall back to deterministic algorithm
        tracing::debug!("ToolRouter: Falling back to deterministic embedding algorithm");
        self.deterministic_embed(text)
    }
    
    /// Generate embeddings using a simple deterministic algorithm
    fn deterministic_embed(&self, text: &str) -> Result<Vec<f32>> {
        // Create a deterministic embedding based on the text content
        // This is a simple algorithm that's not semantically meaningful,
        // but it's deterministic and will work for testing
        let mut embedding = vec![0.0; self.index.dimension];
        
        for (i, byte) in text.bytes().enumerate() {
            let idx = i % self.index.dimension;
            embedding[idx] += (byte as f32) / 255.0;
        }
        
        // Normalize the embedding
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut embedding {
                *val /= norm;
            }
        }
        
        tracing::debug!("ToolRouter: Deterministic embedding generated successfully");
        Ok(embedding)
    }

    /// Activate a specific tool by name
    pub async fn activate_tool(&mut self, name: &str) -> Result<Result<Vec<Content>, ToolError>> {
        tracing::debug!("ToolRouter: Activating tool: {}", name);
        
        if self.tools.contains_key(name) {
            self.active_tools.insert(name.to_string());
            tracing::debug!("ToolRouter: Tool activated successfully");
            Ok(Ok(vec![Content::text(format!("Tool '{}' activated successfully", name))]))
        } else {
            tracing::debug!("ToolRouter: Tool not found: {}", name);
            Ok(Err(ToolError::NotFound(format!("Tool '{}' not found", name))))
        }
    }
    
    /// Deactivate a specific tool by name
    pub async fn deactivate_tool(&mut self, name: &str) -> Result<Result<Vec<Content>, ToolError>> {
        tracing::debug!("ToolRouter: Deactivating tool: {}", name);
        
        if self.active_tools.remove(name) {
            tracing::debug!("ToolRouter: Tool deactivated successfully");
            Ok(Ok(vec![Content::text(format!("Tool '{}' deactivated successfully", name))]))
        } else {
            tracing::debug!("ToolRouter: Tool not active or not found: {}", name);
            Ok(Err(ToolError::NotFound(format!("Tool '{}' not active or not found", name))))
        }
    }
    
    /// Get list of currently active tools
    pub async fn list_active_tools(&self) -> Result<Result<Vec<Content>, ToolError>> {
        tracing::debug!("ToolRouter: Listing active tools");
        
        let active_tools = self.active_tools
            .iter()
            .filter_map(|name| self.tools.get(name))
            .map(|(_, tool)| {
                json!({
                    "name": tool.name,
                    "description": tool.description
                })
            })
            .collect::<Vec<_>>();
            
        tracing::debug!("ToolRouter: Found {} active tools", active_tools.len());
        
        let json_string = serde_json::to_string(&active_tools)?;
        Ok(Ok(vec![Content::text(json_string)]))
    }

    /// Search for tools matching a topic
    pub async fn search_tools(&self, topic: &str) -> Result<Result<Vec<Content>, ToolError>> {
        tracing::debug!("ToolRouter: Starting search for topic: {}", topic);
        
        // Generate embedding for the search query
        tracing::debug!("ToolRouter: Generating embedding for query");
        let query_embedding = self.embed_text(topic).await?;
        tracing::debug!("ToolRouter: Embedding generated successfully");

        // Search the index for similar tools (top 5 matches)
        tracing::debug!("ToolRouter: Searching index with {} tools", self.tools.len());
        let results = self.index.search(&query_embedding, 5);
        tracing::debug!("ToolRouter: Found {} matching tools", results.len());

        // Format results as JSON
        tracing::debug!("ToolRouter: Formatting results as JSON");
        let tool_summaries = results
            .iter()
            .map(|(key, score)| {
                let (extension_name, tool) = self.tools.get(key).unwrap();
                json!({
                    "name": tool.name,
                    "extension": extension_name,
                    "description": tool.description,
                    "is_active": self.active_tools.contains(key),
                    "score": score
                })
            })
            .collect::<Vec<_>>();
        tracing::debug!("ToolRouter: JSON formatting complete");

        let json_string = serde_json::to_string(&tool_summaries)?;
        tracing::debug!("ToolRouter: Returning results: {}", json_string);
        Ok(Ok(vec![Content::text(json_string)]))
    }

    /// Get the full schema for a specific tool
    pub async fn get_tool_schema(&self, name: &str) -> Result<Result<Vec<Content>, ToolError>> {
        if let Some((extension_name, tool)) = self.tools.get(name) {
            Ok(Ok(vec![Content::text(serde_json::to_string(&json!({
                "tool": {
                    "name": tool.name,
                    "extension": extension_name,
                    "description": tool.description,
                    "input_schema": tool.input_schema,
                    "is_active": self.active_tools.contains(name)
                }
            }))?)])) 
        } else {
            Ok(Err(ToolError::NotFound(format!(
                "Tool '{}' not found",
                name
            ))))
        }
    }

    /// Get the tools provided by this extension
    pub fn tools(&self) -> Vec<Tool> {
        vec![
            Tool::new(
                "toolrouter__search_tools",
                "Search for tools matching a topic",
                json!({
                    "type": "object",
                    "properties": {
                        "topic": {
                            "type": "string",
                            "description": "Text query to search for relevant tools"
                        }
                    },
                    "required": ["topic"]
                }),
                None,
            ),
            Tool::new(
                "toolrouter__get_tool_schema",
                "Get full schema for a specific tool",
                json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name of the tool to get schema for"
                        }
                    },
                    "required": ["name"]
                }),
                None,
            ),
            Tool::new(
                "toolrouter__activate_tool",
                "Activate a specific tool by name",
                json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name of the tool to activate"
                        }
                    },
                    "required": ["name"]
                }),
                None,
            ),
            Tool::new(
                "toolrouter__deactivate_tool",
                "Deactivate a specific tool by name",
                json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name of the tool to deactivate"
                        }
                    },
                    "required": ["name"]
                }),
                None,
            ),
            Tool::new(
                "toolrouter__list_active_tools",
                "List all currently active tools",
                json!({
                    "type": "object",
                    "properties": {}
                }),
                None,
            ),
        ]
    }
    
    /// Get only the active tools for a specific extension
    pub fn get_active_tools_for_extension(&self, extension_name: &str) -> Vec<Tool> {
        self.tools
            .iter()
            .filter(|(name, (ext, _))| {
                ext == extension_name && self.active_tools.contains(*name)
            })
            .map(|(_, (_, tool))| tool.clone())
            .collect()
    }
    
    /// Get all active tools
    pub fn get_all_active_tools(&self) -> Vec<Tool> {
        // First, always include the ToolRouter's own tools
        let mut active_tools = self.tools();
        
        // Then add any other active tools
        active_tools.extend(
            self.active_tools
                .iter()
                .filter_map(|name| self.tools.get(name).map(|(_, tool)| tool.clone()))
        );
        
        active_tools
    }

    /// Handle a tool call
    pub async fn handle_call(&mut self, call: ToolCall) -> Result<Result<Vec<Content>, ToolError>> {
        tracing::debug!("ToolRouter: Handling call to tool: {}", call.name);
        
        match call.name.as_str() {
            "toolrouter__search_tools" => {
                tracing::debug!("ToolRouter: Processing search_tools call");
                let topic = call
                    .arguments
                    .get("topic")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'topic' parameter"))?;
                tracing::debug!("ToolRouter: Extracted topic: {}", topic);

                self.search_tools(topic).await
            }
            "toolrouter__get_tool_schema" => {
                tracing::debug!("ToolRouter: Processing get_tool_schema call");
                let name = call
                    .arguments
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'name' parameter"))?;
                tracing::debug!("ToolRouter: Extracted name: {}", name);

                self.get_tool_schema(name).await
            }
            "toolrouter__activate_tool" => {
                tracing::debug!("ToolRouter: Processing activate_tool call");
                let name = call
                    .arguments
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'name' parameter"))?;
                tracing::debug!("ToolRouter: Extracted name: {}", name);

                self.activate_tool(name).await
            }
            "toolrouter__deactivate_tool" => {
                tracing::debug!("ToolRouter: Processing deactivate_tool call");
                let name = call
                    .arguments
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'name' parameter"))?;
                tracing::debug!("ToolRouter: Extracted name: {}", name);

                self.deactivate_tool(name).await
            }
            "toolrouter__list_active_tools" => {
                tracing::debug!("ToolRouter: Processing list_active_tools call");
                self.list_active_tools().await
            }
            _ => {
                tracing::debug!("ToolRouter: Unknown tool: {}", call.name);
                Ok(Err(ToolError::NotFound(format!(
                    "Unknown tool: {}",
                    call.name
                ))))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vector_index() {
        let mut index = VectorIndex::new(3);

        // Add some test embeddings
        index.add("test1", &[1.0, 0.0, 0.0]);
        index.add("test2", &[0.0, 1.0, 0.0]);
        index.add("test3", &[0.0, 0.0, 1.0]);

        // Search for similar embeddings
        let results = index.search(&[1.0, 0.1, 0.1], 2);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "test1");

        // Cosine similarity between [1,0,0] and [1,0.1,0.1] should be high
        assert!(results[0].1 > 0.9);
    }

    #[tokio::test]
    async fn test_cosine_similarity() {
        // Identical vectors should have similarity 1.0
        assert!((cosine_similarity(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]) - 1.0).abs() < 1e-6);

        // Orthogonal vectors should have similarity 0.0
        assert!((cosine_similarity(&[1.0, 0.0, 0.0], &[0.0, 1.0, 0.0]) - 0.0).abs() < 1e-6);

        // Opposite vectors should have similarity -1.0
        assert!((cosine_similarity(&[1.0, 2.0, 3.0], &[-1.0, -2.0, -3.0]) + 1.0).abs() < 1e-6);
    }
}
