import React from "react";
import { Button } from "./ui/button";
import { Bold, Italic, Code, Link, Eye } from "lucide-react";

export interface FloatingToolbarProps {
  style: React.CSSProperties;
  onFormat: (type: string, selectedText: string) => void;
  selectedText: string;
  isPreview: boolean;
  onPreviewToggle: () => void;
}

export const FloatingToolbar = ({ 
  style, 
  onFormat, 
  selectedText, 
  isPreview, 
  onPreviewToggle 
}: FloatingToolbarProps) => {
  const handleButtonClick = (e: React.MouseEvent, type: string) => {
    e.preventDefault();
    e.stopPropagation();
    onFormat(type, selectedText);
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
        onClick={(e) => handleButtonClick(e, 'bold')}
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
        onClick={(e) => handleButtonClick(e, 'italic')}
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
        onClick={(e) => handleButtonClick(e, 'code')}
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
        onClick={(e) => handleButtonClick(e, 'link')}
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