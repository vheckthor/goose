import { tools } from '../tools';
import { ToolCall, ToolCallResult, Content } from '../types/message';
import { submitToolResult } from './toolResult';

export type Tool = {
  name: string;
  description: string;
  function: (args: Record<string, unknown>) => Promise<string>;
  schema: {
    type: string;
    oneOf?: unknown[];
    required?: string[];
    properties?: Record<string, unknown>;
  };
};

export type Tools = Record<string, Tool>;

export async function executeFrontendTool(
  toolCallId: string,
  toolCall: ToolCallResult<ToolCall>
): Promise<void> {
  if (toolCall.status === 'success' && toolCall.value && toolCall.value.name in tools) {
    try {
      const tool = tools[toolCall.value.name as keyof typeof tools];
      const result = await tool.function(toolCall.value.arguments);
      const toolResult: ToolCallResult<Content[]> = {
        status: 'success',
        value: [{
          type: 'text',
          text: result
        }]
      };
      await submitToolResult(toolCallId, toolResult);
    } catch (error) {
      const toolResult: ToolCallResult<Content[]> = {
        status: 'error',
        error: `Error executing tool: ${error instanceof Error ? error.message : String(error)}`
      };
      await submitToolResult(toolCallId, toolResult);
    }
  }
}
