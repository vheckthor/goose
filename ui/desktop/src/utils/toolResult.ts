import { getApiUrl, getSecretKey } from '../config';
import { Content, ToolCallResult } from '../types/message';

export async function submitToolResult(toolCallId: string, result: ToolCallResult<Content[]>) {
  try {
    const response = await fetch(getApiUrl('/tool_result'), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': getSecretKey(),
      },
      body: JSON.stringify({
        id: toolCallId,
        result,
      }),
    });

    if (!response.ok) {
      const errorText = await response.text();
      console.error('Tool result submission error:', errorText);
      throw new Error('Failed to submit tool result');
    }
  } catch (error) {
    console.error('Error submitting tool result:', error);
    throw error;
  }
}
