use anyhow::Result;

use crate::agents::extension_manager::ExtensionManager;
use mcp_core::tool::Tool;

use super::super::agent::Agent;

impl Agent {
    /// Update system prompt and tools after installing extensions
    /// Returns the new system prompt and tool list
    pub async fn update_system_prompt_and_tools_after_install(
        &self,
        extension_manager: &mut ExtensionManager,
    ) -> Result<(String, Vec<Tool>)> {
        let extensions_info = extension_manager.get_extensions_info().await;
        let system_prompt = self
            .prompt_manager
            .build_system_prompt(extensions_info, self.frontend_instructions.clone());

        let tools = extension_manager.get_prefixed_tools().await?;

        Ok((system_prompt, tools))
    }
}
