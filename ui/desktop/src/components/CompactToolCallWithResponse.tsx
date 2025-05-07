import React from 'react';
import { Card } from './ui/card';
import { ToolCallArguments } from './ToolCallArguments';
import MarkdownContent from './MarkdownContent';
import { Content, ToolRequestMessageContent, ToolResponseMessageContent } from '../types/message';
import { snakeToTitleCase } from '../utils';
import Dot, { LoadingStatus } from './ui/Dot';
import Expand from './ui/Expand';

interface CompactToolCallWithResponseProps {
  isCancelledMessage: boolean;
  toolRequest: ToolRequestMessageContent;
  toolResponse?: ToolResponseMessageContent;
}

export default function CompactToolCallWithResponse({
  isCancelledMessage,
  toolRequest,
  toolResponse,
}: CompactToolCallWithResponseProps) {
  const toolCall = toolRequest.toolCall.status === 'success' ? toolRequest.toolCall.value : null;
  if (!toolCall) {
    return null;
  }

  return (
    <div className={'w-full text-textSubtle text-sm'}>
      <Card className="">
        <CompactToolCallView {...{ isCancelledMessage, toolCall, toolResponse }} />
      </Card>
    </div>
  );
}

interface CompactToolCallViewProps {
  isCancelledMessage: boolean;
  toolCall: {
    name: string;
    arguments: Record<string, unknown>;
  };
  toolResponse?: ToolResponseMessageContent;
}

function CompactToolCallView({
  isCancelledMessage,
  toolCall,
  toolResponse,
}: CompactToolCallViewProps) {
  const [isExpanded, setIsExpanded] = React.useState(false);
  const toggleExpand = () => setIsExpanded((prev) => !prev);

  const loadingStatus: LoadingStatus = !toolResponse?.toolResult.status
    ? 'loading'
    : toolResponse?.toolResult.status;

  const toolResults: { result: Content; defaultExpanded: boolean }[] =
    loadingStatus === 'success' && Array.isArray(toolResponse?.toolResult.value)
      ? toolResponse.toolResult.value
          .filter((item) => {
            const audience = item.annotations?.audience as string[] | undefined;
            return !audience || audience.includes('user');
          })
          .map((item) => ({
            result: item,
            defaultExpanded: false, // Always collapsed by default
          }))
      : [];

  // Create a compact summary of the parameters
  const paramSummary = getParameterSummary(toolCall.arguments);

  return (
    <div className="w-full">
      {/* Compact Header - Always Visible */}
      <div className="flex justify-between items-center w-full pr-2">
        <div className="flex items-center flex-grow overflow-hidden">
          <Dot size={2} loadingStatus={loadingStatus} />
          <span className="ml-[10px] font-medium">
            {snakeToTitleCase(toolCall.name.substring(toolCall.name.lastIndexOf('__') + 2))}
          </span>
          {/* Compact parameter summary */}
          <span className="ml-2 text-textSubtle truncate text-xs opacity-70">{paramSummary}</span>
        </div>
        <button onClick={toggleExpand} className="flex-shrink-0 hover:opacity-75">
          <Expand size={5} isExpanded={isExpanded} />
        </button>
      </div>

      {/* Expanded Content */}
      {isExpanded && (
        <div className="mt-2">
          {/* Tool Details */}
          {Object.entries(toolCall.arguments).length > 0 && (
            <div className="bg-bgStandard rounded-t mt-1 p-2">
              <div className="text-xs font-medium mb-1">Tool Details</div>
              <ToolCallArguments args={toolCall.arguments} />
            </div>
          )}

          {/* Tool Output */}
          {!isCancelledMessage && toolResults.length > 0 && (
            <div className="bg-bgStandard mt-1 p-2">
              <div className="text-xs font-medium mb-1">Output</div>
              {toolResults.map(({ result }, index) => (
                <div key={index} className="bg-bgApp rounded p-2 mt-1">
                  {result.type === 'text' && result.text && (
                    <MarkdownContent
                      content={result.text}
                      className="whitespace-pre-wrap max-w-full overflow-x-auto"
                    />
                  )}
                  {result.type === 'image' && (
                    <img
                      src={`data:${result.mimeType};base64,${result.data}`}
                      alt="Tool result"
                      className="max-w-full h-auto rounded-md my-2"
                      onError={(e) => {
                        console.error('Failed to load image');
                        e.currentTarget.style.display = 'none';
                      }}
                    />
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// Helper function to create a compact summary of parameters
function getParameterSummary(args: Record<string, unknown>): string {
  const entries = Object.entries(args);
  if (entries.length === 0) return '';

  // For a single parameter, show key and truncated value
  if (entries.length === 1) {
    const [key, value] = entries[0];
    const stringValue = typeof value === 'string' ? value : JSON.stringify(value);

    const truncatedValue =
      stringValue.length > 30 ? stringValue.substring(0, 30) + '...' : stringValue;

    return `${key}: ${truncatedValue}`;
  }

  // For multiple parameters, just show the keys
  return entries.map(([key]) => key).join(', ');
}
