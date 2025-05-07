import React from 'react';
import { useToolCallViewMode } from './ToolCallViewMode';
import OriginalToolCallWithResponse from './ToolCallWithResponse';
import CompactToolCallWithResponse from './CompactToolCallWithResponse';
import { ToolRequestMessageContent, ToolResponseMessageContent } from '../types/message';

interface AdaptiveToolCallWithResponseProps {
  isCancelledMessage: boolean;
  toolRequest: ToolRequestMessageContent;
  toolResponse?: ToolResponseMessageContent;
}

/**
 * A wrapper component that renders either the compact or expanded tool call view
 * based on the user's preference.
 */
export default function AdaptiveToolCallWithResponse(props: AdaptiveToolCallWithResponseProps) {
  const { isCompactMode } = useToolCallViewMode();

  return isCompactMode ? (
    <CompactToolCallWithResponse {...props} />
  ) : (
    <OriginalToolCallWithResponse {...props} />
  );
}
