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
          const openingIndex = beforeText.lastIndexOf('**');
          const closingIndex = afterText.indexOf('**') + selectionEnd;
          if (openingIndex !== -1 && closingIndex !== -1) {
            rangeStart = openingIndex;
            rangeEnd = closingIndex + 2;
          }
        }
        break;
        
      case 'italic':
        if (beforeText.endsWith('*') && !beforeText.endsWith('**') && 
            afterText.startsWith('*') && !afterText.startsWith('**')) {
          const openingIndex = beforeText.lastIndexOf('*');
          const closingIndex = afterText.indexOf('*') + selectionEnd;
          if (openingIndex !== -1 && closingIndex !== -1) {
            rangeStart = openingIndex;
            rangeEnd = closingIndex + 1;
          }
        }
        break;
        
      case 'code':
        if (beforeText.endsWith('```\n') && afterText.startsWith('\n```')) {
          const openingIndex = beforeText.lastIndexOf('```\n');
          const closingIndex = afterText.indexOf('\n```') + selectionEnd;
          if (openingIndex !== -1 && closingIndex !== -1) {
            rangeStart = openingIndex;
            rangeEnd = closingIndex + 4;
          }
        }
        break;
        
      case 'link':
        const beforeLink = beforeText.lastIndexOf('[');
        const afterCloseBracket = afterText.indexOf(']');
        const afterOpenParen = afterText.indexOf('](', afterCloseBracket);
        const afterCloseParen = afterText.indexOf(')', afterOpenParen);
        
        if (beforeLink !== -1 && afterCloseBracket !== -1 && 
            afterOpenParen !== -1 && afterCloseParen !== -1) {
          rangeStart = beforeLink;
          rangeEnd = selectionEnd + afterCloseParen + 1;
        }
        break;
    }
    
    return { rangeStart, rangeEnd };
  };

  const handleFormat = (e: React.MouseEvent, type: string) => {
    e.preventDefault();
    e.stopPropagation();

    const { rangeStart, rangeEnd } = getFormattedTextRange(type);
    const selectedContent = value.substring(rangeStart, rangeEnd);
    
    let newText: string;
    let newSelectionStart: number;
    let newSelectionEnd: number;

    switch (type) {
      case 'bold': {
        const isBold = /^\*\*(.*)\*\*$/.test(selectedContent);
        if (isBold) {
          const unformattedText = selectedContent.slice(2, -2);
          newText = value.substring(0, rangeStart) + unformattedText + value.substring(rangeEnd);
          newSelectionStart = rangeStart;
          newSelectionEnd = rangeStart + unformattedText.length;
        } else {
          newText = value.substring(0, selectionStart) + `**${selectedText}**` + value.substring(selectionEnd);
          newSelectionStart = selectionStart + 2;
          newSelectionEnd = selectionStart + selectedText.length + 2;
        }
        break;
      }
      
      case 'italic': {
        const isItalic = /^\*((?!\*).)*\*$/.test(selectedContent);
        if (isItalic) {
          const unformattedText = selectedContent.slice(1, -1);
          newText = value.substring(0, rangeStart) + unformattedText + value.substring(rangeEnd);
          newSelectionStart = rangeStart;
          newSelectionEnd = rangeStart + unformattedText.length;
        } else {
          newText = value.substring(0, selectionStart) + `*${selectedText}*` + value.substring(selectionEnd);
          newSelectionStart = selectionStart + 1;
          newSelectionEnd = selectionStart + selectedText.length + 1;
        }
        break;
      }
      
      case 'code': {
        const isCode = /^```\n([\s\S]*)\n```$/.test(selectedContent);
        if (isCode) {
          const unformattedText = selectedContent.slice(4, -4);
          newText = value.substring(0, rangeStart) + unformattedText + value.substring(rangeEnd);
          newSelectionStart = rangeStart;
          newSelectionEnd = rangeStart + unformattedText.length;
        } else {
          newText = value.substring(0, selectionStart) + `\`\`\`\n${selectedText}\n\`\`\`\n` + value.substring(selectionEnd);
          newSelectionStart = selectionStart + 4;
          newSelectionEnd = selectionStart + selectedText.length + 4;
        }
        break;
      }
      
      case 'link': {
        const isLink = /^\[(.*)\]\((.*)\)$/.test(selectedContent);
        if (isLink) {
          const linkText = selectedContent.match(/^\[(.*)\]\((.*)\)$/);
          if (linkText) {
            newText = value.substring(0, rangeStart) + linkText[1] + value.substring(rangeEnd);
            newSelectionStart = rangeStart;
            newSelectionEnd = rangeStart + linkText[1].length;
          } else {
            return; // Invalid link format
          }
        } else {
          newText = value.substring(0, selectionStart) + `[${selectedText}]()` + value.substring(selectionEnd);
          newSelectionStart = selectionStart + selectedText.length + 3; // Position cursor in ()
          newSelectionEnd = newSelectionStart;
        }
        break;
      }
      
      default:
        return;
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