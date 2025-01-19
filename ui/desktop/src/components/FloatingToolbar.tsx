import React from "react";
import { Button } from "./ui/button";
import { Bold, Italic, Code, Link, Eye } from "lucide-react";

export interface FloatingToolbarProps {
  style: React.CSSProperties;
  value: string;
  selectionStart: number;
  selectionEnd: number;
  onTextChange: (newText: string, newSelectionStart: number, newSelectionEnd: number) => void;
  selectedText: string;
  isPreview: boolean;
  onPreviewToggle: () => void;
}

export const FloatingToolbar = ({ 
  style, 
  value,
  selectionStart,
  selectionEnd,
  onTextChange,
  selectedText, 
  isPreview, 
  onPreviewToggle 
}: FloatingToolbarProps) => {
  // Helper function to check if text is already formatted
  const isAlreadyFormatted = (text: string, format: string) => {
    switch (format) {
      case 'bold':
        return text.startsWith('**') && text.endsWith('**');
      case 'italic':
        return text.startsWith('*') && text.endsWith('*') && !text.startsWith('**');
      case 'code':
        return text.startsWith('```\n') && text.endsWith('\n```');
      case 'link':
        return text.match(/^\[.*\]\(.*\)$/);
      default:
        return false;
    }
  };

  // Get the full text that might be formatted (including markers)
  const getFormattedTextRange = (format: string) => {
    let rangeStart = selectionStart;
    let rangeEnd = selectionEnd;
    const beforeText = value.substring(0, selectionStart);
    const afterText = value.substring(selectionEnd);

    switch (format) {
      case 'bold':
        if (beforeText.endsWith('**') && afterText.startsWith('**')) {
          rangeStart -= 2;
          rangeEnd += 2;
        }
        break;
      case 'italic':
        if (beforeText.endsWith('*') && afterText.startsWith('*')) {
          rangeStart -= 1;
          rangeEnd += 1;
        }
        break;
      case 'code':
        if (beforeText.endsWith('```\n') && afterText.startsWith('\n```')) {
          rangeStart -= 4;
          rangeEnd += 4;
        }
        break;
      case 'link':
        const beforeLink = beforeText.match(/\[$/);
        const afterLink = afterText.match(/^\]\(\)/);
        if (beforeLink && afterLink) {
          rangeStart -= 1;
          rangeEnd += 3;
        }
        break;
    }
    return { rangeStart, rangeEnd };
  };

  const handleFormat = (e: React.MouseEvent, type: string) => {
    e.preventDefault();
    e.stopPropagation();

    const { rangeStart, rangeEnd } = getFormattedTextRange(type);
    const possiblyFormattedText = value.substring(rangeStart, rangeEnd);
    let newText = value;
    let newSelectionStart = selectionStart;
    let newSelectionEnd = selectionEnd;

    if (isAlreadyFormatted(possiblyFormattedText, type)) {
      // Remove formatting
      switch (type) {
        case 'bold':
          newText = value.substring(0, rangeStart) + possiblyFormattedText.slice(2, -2) + value.substring(rangeEnd);
          newSelectionStart = rangeStart;
          newSelectionEnd = rangeEnd - 4;
          break;
        case 'italic':
          newText = value.substring(0, rangeStart) + possiblyFormattedText.slice(1, -1) + value.substring(rangeEnd);
          newSelectionStart = rangeStart;
          newSelectionEnd = rangeEnd - 2;
          break;
        case 'code':
          newText = value.substring(0, rangeStart) + possiblyFormattedText.slice(4, -4) + value.substring(rangeEnd);
          newSelectionStart = rangeStart;
          newSelectionEnd = rangeEnd - 8;
          break;
        case 'link':
          const linkText = possiblyFormattedText.match(/^\[(.*)\]\((.*)\)$/);
          if (linkText) {
            newText = value.substring(0, rangeStart) + linkText[1] + value.substring(rangeEnd);
            newSelectionStart = rangeStart;
            newSelectionEnd = rangeStart + linkText[1].length;
          }
          break;
      }
    } else {
      // Add formatting
      switch (type) {
        case 'bold':
          newText = value.substring(0, selectionStart) + `**${selectedText}**` + value.substring(selectionEnd);
          newSelectionStart = selectionStart + 2;
          newSelectionEnd = selectionStart + 2 + selectedText.length;
          break;
        case 'italic':
          newText = value.substring(0, selectionStart) + `*${selectedText}*` + value.substring(selectionEnd);
          newSelectionStart = selectionStart + 1;
          newSelectionEnd = selectionStart + 1 + selectedText.length;
          break;
        case 'code':
          newText = value.substring(0, selectionStart) + `\n\`\`\`\n${selectedText}\n\`\`\`\n` + value.substring(selectionEnd);
          newSelectionStart = selectionStart + 5;
          newSelectionEnd = selectionStart + 5 + selectedText.length;
          break;
        case 'link':
          newText = value.substring(0, selectionStart) + `[${selectedText}]()` + value.substring(selectionEnd);
          newSelectionStart = selectionStart + 1;
          newSelectionEnd = selectionStart + 1 + selectedText.length;
          break;
      }
    }

    onTextChange(newText, newSelectionStart, newSelectionEnd);
  };

  return (
    <div 
      className="absolute flex items-center gap-2 px-2.5 py-1.5 rounded-[1000px] bg-black/5 dark:bg-white/5 hover:bg-black/10 dark:hover:bg-white/10 transition-all duration-150 backdrop-blur-sm"
      style={style}
    >
      <Button 
        size="icon" 
        variant="ghost" 
        className={`h-7 w-7 text-black/70 dark:text-white/70 bg-transparent hover:bg-black/10 dark:hover:bg-white/10 ${
          isPreview ? 'opacity-50 cursor-not-allowed' : ''
        }`}
        onClick={(e) => handleFormat(e, 'bold')}
        disabled={isPreview}
      >
        <Bold className="h-4 w-4" />
      </Button>
      <Button 
        size="icon" 
        variant="ghost" 
        className={`h-7 w-7 text-black/70 dark:text-white/70 bg-transparent hover:bg-black/10 dark:hover:bg-white/10 ${
          isPreview ? 'opacity-50 cursor-not-allowed' : ''
        }`}
        onClick={(e) => handleFormat(e, 'italic')}
        disabled={isPreview}
      >
        <Italic className="h-4 w-4" />
      </Button>
      <Button 
        size="icon" 
        variant="ghost" 
        className={`h-7 w-7 text-black/70 dark:text-white/70 bg-transparent hover:bg-black/10 dark:hover:bg-white/10 ${
          isPreview ? 'opacity-50 cursor-not-allowed' : ''
        }`}
        onClick={(e) => handleFormat(e, 'code')}
        disabled={isPreview}
      >
        <Code className="h-4 w-4" />
      </Button>
      <Button 
        size="icon" 
        variant="ghost" 
        className={`h-7 w-7 text-black/70 dark:text-white/70 bg-transparent hover:bg-black/10 dark:hover:bg-white/10 ${
          isPreview ? 'opacity-50 cursor-not-allowed' : ''
        }`}
        onClick={(e) => handleFormat(e, 'link')}
        disabled={isPreview}
      >
        <Link className="h-4 w-4" />
      </Button>
      <Button 
        size="icon" 
        variant="ghost" 
        className={`h-7 w-7 text-black/70 dark:text-white/70 bg-transparent hover:bg-black/10 dark:hover:bg-white/10 ${
          isPreview ? 'bg-black/10 dark:bg-white/10' : ''
        }`}
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          onPreviewToggle();
        }}
      >
        <Eye className="h-4 w-4" />
      </Button>
    </div>
  );
};