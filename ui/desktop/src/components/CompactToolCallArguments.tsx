import React, { useState } from 'react';
import MarkdownContent from './MarkdownContent';
import Expand from './ui/Expand';

type ToolCallArgumentValue =
  | string
  | number
  | boolean
  | null
  | ToolCallArgumentValue[]
  | { [key: string]: ToolCallArgumentValue };

interface CompactToolCallArgumentsProps {
  args: Record<string, ToolCallArgumentValue>;
}

export function CompactToolCallArguments({ args }: CompactToolCallArgumentsProps) {
  const [expandedKeys, setExpandedKeys] = useState<Record<string, boolean>>({});

  const toggleKey = (key: string) => {
    setExpandedKeys((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  const renderValue = (key: string, value: ToolCallArgumentValue) => {
    if (typeof value === 'string') {
      const needsExpansion = value.length > 40; // Reduced from 60 to 40 for more compact display
      const isExpanded = expandedKeys[key];

      if (!needsExpansion) {
        return (
          <div className="text-xs mb-1">
            <div className="flex flex-row">
              <span className="text-textSubtle min-w-[100px] font-medium">{key}</span>
              <span className="text-textPlaceholder">{value}</span>
            </div>
          </div>
        );
      }

      return (
        <div className="text-xs mb-1">
          <div className="flex flex-row">
            <span className="text-textSubtle min-w-[100px] font-medium">{key}</span>
            <div className="w-full flex justify-between items-start">
              {isExpanded ? (
                <div className="">
                  <MarkdownContent content={value} className="text-xs text-textPlaceholder" />
                </div>
              ) : (
                <span className="text-textPlaceholder mr-2">{value.slice(0, 40)}...</span>
              )}
              <button
                onClick={() => toggleKey(key)}
                className="hover:opacity-75 text-textPlaceholder"
              >
                <Expand size={4} isExpanded={isExpanded} />
              </button>
            </div>
          </div>
        </div>
      );
    }

    // Handle non-string values (arrays, objects, etc.)
    const isComplex = typeof value === 'object' && value !== null;
    const isExpanded = expandedKeys[key];

    // Create a compact representation for complex objects
    const compactContent = Array.isArray(value)
      ? `[${value.length} items]`
      : isComplex
        ? `{${Object.keys(value as object).length} keys}`
        : String(value);

    // Full content for when expanded
    const fullContent = Array.isArray(value)
      ? value.map((item, index) => `${index + 1}. ${JSON.stringify(item)}`).join('\n')
      : isComplex
        ? JSON.stringify(value, null, 2)
        : String(value);

    return (
      <div className="text-xs mb-1">
        <div className="flex flex-row">
          <span className="text-textSubtle min-w-[100px] font-medium">{key}</span>
          <div className="w-full flex justify-between items-start">
            {isComplex ? (
              <>
                {isExpanded ? (
                  <pre className="whitespace-pre-wrap text-textPlaceholder text-xs">
                    {fullContent}
                  </pre>
                ) : (
                  <span className="text-textPlaceholder">{compactContent}</span>
                )}
                <button
                  onClick={() => toggleKey(key)}
                  className="hover:opacity-75 text-textPlaceholder ml-2"
                >
                  <Expand size={4} isExpanded={isExpanded} />
                </button>
              </>
            ) : (
              <span className="text-textPlaceholder">{compactContent}</span>
            )}
          </div>
        </div>
      </div>
    );
  };

  return (
    <div className="my-1">
      {Object.entries(args).map(([key, value]) => (
        <div key={key}>{renderValue(key, value)}</div>
      ))}
    </div>
  );
}
