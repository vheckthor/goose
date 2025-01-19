import React, { useRef, useState, useEffect } from "react";
import { Button } from "./ui/button";
import Stop from "./ui/Stop";
import { Attach, Send } from "./icons";
import { Bold, Italic, Code, Link } from "lucide-react";

interface InputProps {
  handleSubmit: (e: React.FormEvent) => void;
  disabled?: boolean;
  isLoading?: boolean;
  onStop?: () => void;
}

interface SelectionCoords {
  x: number;
  y: number;
  absoluteY: number;
  scrollTop: number;
  isPinned?: boolean;
}

declare global {
  interface Window {
    electron: {
      selectFileOrDirectory: () => Promise<string | null>;
    };
  }
}

interface FloatingToolbarProps {
  style: React.CSSProperties;
  onFormat: (type: string, selectedText: string) => void;
  selectedText: string;
}

const FloatingToolbar = ({ style, onFormat, selectedText }: FloatingToolbarProps) => {
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
        className="h-7 w-7 text-black/70 dark:text-white/70 bg-transparent hover:bg-black/10 dark:hover:bg-white/10"
        onClick={(e) => handleButtonClick(e, 'bold')}
      >
        <Bold className="h-4 w-4" />
      </Button>
      <Button 
        size="icon" 
        variant="ghost" 
        className="h-7 w-7 text-black/70 dark:text-white/70 bg-transparent hover:bg-black/10 dark:hover:bg-white/10"
        onClick={(e) => handleButtonClick(e, 'italic')}
      >
        <Italic className="h-4 w-4" />
      </Button>
      <Button 
        size="icon" 
        variant="ghost" 
        className="h-7 w-7 text-black/70 dark:text-white/70 bg-transparent hover:bg-black/10 dark:hover:bg-white/10"
        onClick={(e) => handleButtonClick(e, 'code')}
      >
        <Code className="h-4 w-4" />
      </Button>
      <Button 
        size="icon" 
        variant="ghost" 
        className="h-7 w-7 text-black/70 dark:text-white/70 bg-transparent hover:bg-black/10 dark:hover:bg-white/10"
        onClick={(e) => handleButtonClick(e, 'link')}
      >
        <Link className="h-4 w-4" />
      </Button>
    </div>
  );
};

export default function Input({
  handleSubmit,
  disabled = false,
  isLoading = false,
  onStop,
}: InputProps) {
  const [value, setValue] = useState("");
  const textAreaRef = useRef<HTMLTextAreaElement>(null);
  const editorRef = useRef<HTMLDivElement>(null);
  const [selectionCoords, setSelectionCoords] = useState<SelectionCoords | null>(null);

  useEffect(() => {
    if (textAreaRef.current && !disabled) {
      textAreaRef.current.focus();
    }
  }, [disabled, value]);

  const updateSelection = () => {
    const textarea = textAreaRef.current;
    const editor = editorRef.current;
    if (!textarea || !editor) return;

    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    
    if (start === end) {
      setSelectionCoords(null);
      return;
    }

    // Copy text and selection to contenteditable div
    editor.textContent = textarea.value;
    
    // Ensure editor has same dimensions and styles as textarea
    const textareaStyles = window.getComputedStyle(textarea);
    editor.style.width = textareaStyles.width;
    editor.style.height = textareaStyles.height;
    editor.style.lineHeight = textareaStyles.lineHeight;
    editor.style.fontSize = textareaStyles.fontSize;
    editor.style.fontFamily = textareaStyles.fontFamily;
    editor.style.padding = textareaStyles.padding;
    editor.style.boxSizing = textareaStyles.boxSizing;
    editor.style.borderWidth = textareaStyles.borderWidth;
    editor.style.whiteSpace = 'pre-wrap';
    editor.style.wordWrap = 'break-word';
    editor.style.display = 'block';
    editor.style.position = 'absolute';
    editor.style.top = '0';
    editor.style.left = '0';

    // Create range for the selection
    const range = document.createRange();
    const textNode = editor.firstChild;
    
    if (textNode) {
      range.setStart(textNode, start);
      range.setEnd(textNode, end);
      
      const rects = range.getClientRects();
      if (rects.length > 0) {
        const editorRect = editor.getBoundingClientRect();
        const firstRect = rects[0];
        const toolbarWidth = 200; // Approximate width of toolbar
        const toolbarHeight = 40; // Approximate height of toolbar
        
        // Calculate position relative to editor
        let x = firstRect.left - editorRect.left;
        let y = firstRect.top - editorRect.top - textarea.scrollTop;
        let isPinned = false;
        
        // Check boundaries
        if (y < 0) {
          y = -4;
          x = 0;
          isPinned = true;
        }
        
        // Check right boundary
        if (x + toolbarWidth > editorRect.width) {
          x = editorRect.width - toolbarWidth - 4;
        }
        
        // Check bottom boundary
        if (y + toolbarHeight > editorRect.height) {
          y = editorRect.height - toolbarHeight - 4;
        }

        setSelectionCoords({
          x,
          y,
          absoluteY: firstRect.top - editorRect.top,
          scrollTop: textarea.scrollTop,
          isPinned
        });
      }
    }

    editor.style.display = 'none';
  };

  const handleScroll = (e: React.UIEvent<HTMLTextAreaElement>) => {
    if (selectionCoords) {
      let y = selectionCoords.absoluteY - e.currentTarget.scrollTop;
      let x = selectionCoords.x;
      let isPinned = false;

      // If y would be negative, pin to top-left
      if (y < 0) {
        y = -4; // Position slightly above the border
        x = 0;
        isPinned = true;
      }

      const newCoords = {
        ...selectionCoords,
        x,
        y,
        scrollTop: e.currentTarget.scrollTop,
        isPinned
      };
      
      setSelectionCoords(newCoords);
    }
  };

  const useAutosizeTextArea = (
    textAreaRef: HTMLTextAreaElement | null,
    value: string
  ) => {
    useEffect(() => {
      if (textAreaRef) {
        textAreaRef.style.height = "0px"; // Reset height
        const scrollHeight = textAreaRef.scrollHeight;
        textAreaRef.style.height = Math.min(scrollHeight, maxHeight) + "px";
      }
    }, [textAreaRef, value]);
  };

  const minHeight = "1rem";
  const maxHeight = 10 * 24;

  useAutosizeTextArea(textAreaRef.current, value);

  const handleChange = (evt: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = evt.target?.value;
    setValue(val);
  };

  const handleKeyDown = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (evt.key === "Enter" && !evt.shiftKey) {
      evt.preventDefault();
      if (value.trim()) {
        handleSubmit(new CustomEvent("submit", { detail: { value } }));
        setValue("");
      }
    }
  };

  const onFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (value.trim()) {
      handleSubmit(new CustomEvent("submit", { detail: { value } }));
      setValue("");
    }
  };

  const handleFileSelect = async () => {
    const path = await window.electron.selectFileOrDirectory();
    if (path) {
      setValue(path);
      textAreaRef.current?.focus();
    }
  };

  const handleFormat = (type: string, selectedText: string) => {
    const textarea = textAreaRef.current;
    if (!textarea) return;

    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    let newText = value;
    let newCursorPos = start;

    switch (type) {
      case 'bold':
        newText = value.substring(0, start) + `**${selectedText}**` + value.substring(end);
        newCursorPos = start + 2 + selectedText.length;
        break;
      case 'italic':
        newText = value.substring(0, start) + `*${selectedText}*` + value.substring(end);
        newCursorPos = start + 1 + selectedText.length;
        break;
      case 'code':
        newText = value.substring(0, start) + `\n\`\`\`\n${selectedText}\n\`\`\`\n` + value.substring(end);
        newCursorPos = start + 5 + selectedText.length;
        break;
      case 'link':
        newText = value.substring(0, start) + `[${selectedText}]()` + value.substring(end);
        newCursorPos = start + selectedText.length + 3;
        break;
    }

    setValue(newText);
    
    // Use setTimeout to ensure the new value is set before we try to focus and select
    setTimeout(() => {
      if (textarea) {
        textarea.focus();
        textarea.setSelectionRange(newCursorPos, newCursorPos);
      }
    }, 0);
  };

  return (
    <form
      onSubmit={onFormSubmit}
      className="flex relative h-auto px-[16px] pr-[68px] py-[1rem] border-t dark:border-gray-700"
    >
      <div className="relative flex-1">
        <textarea
          autoFocus
          id="dynamic-textarea"
          placeholder="What should goose do?"
          value={value}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          onSelect={updateSelection}
          onScroll={handleScroll}
          disabled={disabled}
          ref={textAreaRef}
          rows={1}
          style={{
            minHeight: `${minHeight}px`,
            maxHeight: `${maxHeight}px`,
            overflowY: "auto",
            whiteSpace: 'pre-wrap',
            wordWrap: 'break-word'
          }}
          className={`w-full outline-none border-none focus:ring-0 bg-transparent p-0 text-14 resize-none ${
            disabled ? "cursor-not-allowed opacity-50" : ""
          }`}
        />
        
        {/* Hidden editor for measuring selection */}
        <div
          ref={editorRef}
          className="absolute top-0 left-0 w-full invisible pointer-events-none whitespace-pre-wrap break-words"
          style={{
            font: 'inherit',
            display: 'none'
          }}
        />
        
        {selectionCoords && (
          <FloatingToolbar 
            style={{
              left: `${selectionCoords.x}px`,
              top: `${selectionCoords.y}px`,
              transform: 'translateY(-112%)',
            }}
            onFormat={handleFormat}
            selectedText={value.substring(textAreaRef.current?.selectionStart || 0, textAreaRef.current?.selectionEnd || 0)}
          />
        )}
      </div>

      <Button
        type="button"
        size="icon"
        variant="ghost"
        onClick={handleFileSelect}
        disabled={disabled}
        className={`absolute right-[40px] top-1/2 -translate-y-1/2 text-indigo-600 dark:text-indigo-300 hover:text-indigo-700 dark:hover:text-indigo-200 hover:bg-indigo-100 dark:hover:bg-indigo-800 ${
          disabled ? "opacity-50 cursor-not-allowed" : ""
        }`}
      >
        <Attach />
      </Button>
      {isLoading ? (
        <Button
          type="button"
          size="icon"
          variant="ghost"
          onClick={onStop}
          className="absolute right-2 top-1/2 -translate-y-1/2 bg-indigo-100 dark:bg-indigo-800 dark:text-indigo-200 text-indigo-600 hover:opacity-50 [&_svg]:size-5"
        >
          <Stop size={24} />
        </Button>
      ) : (
        <Button
          type="submit"
          size="icon"
          variant="ghost"
          disabled={disabled || !value.trim()}
          className={`absolute right-2 top-1/2 -translate-y-1/2 text-indigo-600 dark:text-indigo-300 hover:text-indigo-700 dark:hover:text-indigo-200 hover:bg-indigo-100 dark:hover:bg-indigo-800 ${
            disabled || !value.trim() ? "opacity-50 cursor-not-allowed" : ""
          }`}
        >
          <Send />
        </Button>
      )}
    </form>
  );
}