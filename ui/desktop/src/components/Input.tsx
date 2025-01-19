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

const FloatingToolbar = ({ style }: { style: React.CSSProperties }) => {
  return (
    <div 
      className="absolute flex items-center gap-3 px-3 py-2 rounded-[1000px] bg-black/5 dark:bg-white/5 transition-all duration-150"
      style={style}
    >
      <Button size="icon" variant="ghost" className="h-8 w-8 text-black/60 dark:text-white/60 bg-transparent hover:bg-black/10 dark:hover:bg-white/10">
        <Bold className="h-4 w-4" />
      </Button>
      <Button size="icon" variant="ghost" className="h-8 w-8 text-black/60 dark:text-white/60 bg-transparent hover:bg-black/10 dark:hover:bg-white/10">
        <Italic className="h-4 w-4" />
      </Button>
      <Button size="icon" variant="ghost" className="h-8 w-8 text-black/60 dark:text-white/60 bg-transparent hover:bg-black/10 dark:hover:bg-white/10">
        <Code className="h-4 w-4" />
      </Button>
      <Button size="icon" variant="ghost" className="h-8 w-8 text-black/60 dark:text-white/60 bg-transparent hover:bg-black/10 dark:hover:bg-white/10">
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
      
      // Get selection coordinates
      const rects = range.getClientRects();
      if (rects.length > 0) {
        const editorRect = editor.getBoundingClientRect();
        const firstRect = rects[0];
        
        // Calculate position relative to editor
        let x = firstRect.left - editorRect.left;
        let y = firstRect.top - editorRect.top - textarea.scrollTop;
        let isPinned = false;
        
        // If y would be negative (dot would appear above form), 
        // pin to top-left of form
        if (y < 0) {
          y = -4;  // Position slightly above the border
          x = 0;  // Pin to left
          isPinned = true;
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
              transform: 'translateY(-125%)',
            }}
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