import React, { useRef, useState, useEffect } from 'react';
import { Button } from './ui/button';
import Stop from './ui/Stop';
import type { View } from '../App';
import { Attach, ChevronDown, Send, Document, ChevronUp } from './icons';
import ModelsBottomBar from './settings_v2/models/bottom_bar/ModelsBottomBar';
import { BottomMenuModeSelection } from './BottomMenuModeSelection';

interface InputProps {
  handleSubmit: (e: React.FormEvent) => void;
  isLoading?: boolean;
  onStop?: () => void;
  commandHistory?: string[];
  initialValue?: string;
  setView: (view: View) => void;
  hasMessages?: boolean;
}

export default function Input({
  handleSubmit,
  isLoading = false,
  onStop,
  commandHistory = [],
  initialValue = '',
  setView,
  hasMessages = false,
}: InputProps) {
  const [value, setValue] = useState(initialValue);
  const [isFocused, setIsFocused] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Update internal value when initialValue changes
  useEffect(() => {
    if (initialValue) {
      setValue(initialValue);
    }
  }, [initialValue]);

  // State to track if the IME is composing (i.e., in the middle of Japanese IME input)
  const [isComposing, setIsComposing] = useState(false);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [savedInput, setSavedInput] = useState('');
  const textAreaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (textAreaRef.current) {
      textAreaRef.current.focus();
    }
  }, []);

  const useAutosizeTextArea = (textAreaRef: HTMLTextAreaElement | null, value: string) => {
    useEffect(() => {
      if (textAreaRef) {
        textAreaRef.style.height = '0px'; // Reset height
        const scrollHeight = textAreaRef.scrollHeight;
        textAreaRef.style.height = Math.min(scrollHeight, maxHeight) + 'px';
      }
    }, [textAreaRef, value]);
  };

  const minHeight = '1rem';
  const maxHeight = 10 * 24;

  useAutosizeTextArea(textAreaRef.current, value);

  const handleChange = (evt: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = evt.target.value;
    setValue(val);
  };

  // Handlers for composition events, which are crucial for proper IME behavior
  const handleCompositionStart = (evt: React.CompositionEvent<HTMLTextAreaElement>) => {
    setIsComposing(true);
  };

  const handleCompositionEnd = (evt: React.CompositionEvent<HTMLTextAreaElement>) => {
    setIsComposing(false);
  };

  const handleHistoryNavigation = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    evt.preventDefault();

    // Save current input if we're just starting to navigate history
    if (historyIndex === -1) {
      setSavedInput(value);
    }

    // Calculate new history index
    let newIndex = historyIndex;
    if (evt.key === 'ArrowUp') {
      // Move backwards through history
      if (historyIndex < commandHistory.length - 1) {
        newIndex = historyIndex + 1;
      }
    } else {
      // Move forwards through history
      if (historyIndex > -1) {
        newIndex = historyIndex - 1;
      }
    }

    if (newIndex == historyIndex) {
      return;
    }

    // Update index and value
    setHistoryIndex(newIndex);
    if (newIndex === -1) {
      // Restore saved input when going past the end of history
      setValue(savedInput);
    } else {
      setValue(commandHistory[newIndex] || '');
    }
  };

  const handleKeyDown = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // Handle command history navigation
    if ((evt.metaKey || evt.ctrlKey) && (evt.key === 'ArrowUp' || evt.key === 'ArrowDown')) {
      handleHistoryNavigation(evt);
      return;
    }

    if (evt.key === 'Enter') {
      // should not trigger submit on Enter if it's composing (IME input in progress) or shift/alt(option) is pressed
      if (evt.shiftKey || isComposing) {
        // Allow line break for Shift+Enter, or during IME composition
        return;
      }
      if (evt.altKey) {
        setValue(value + '\n');
        return;
      }

      // Prevent default Enter behavior when loading or when not loading but has content
      // So it won't trigger a new line
      evt.preventDefault();

      // Only submit if not loading and has content
      if (!isLoading && value.trim()) {
        handleSubmit(new CustomEvent('submit', { detail: { value } }));
        setValue('');
        setHistoryIndex(-1);
        setSavedInput('');
      }
    }
  };

  const onFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (value.trim() && !isLoading) {
      handleSubmit(new CustomEvent('submit', { detail: { value } }));
      setValue('');
      setHistoryIndex(-1);
      setSavedInput('');
    }
  };

  const handleFileSelect = async () => {
    const path = await window.electron.selectFileOrDirectory();
    if (path) {
      // Append the path to existing text, with a space if there's existing text
      setValue((prev) => {
        const currentText = prev.trim();
        return currentText ? `${currentText} ${path}` : path;
      });
      textAreaRef.current?.focus();
    }
  };

  return (
    <form
      onSubmit={onFormSubmit}
      className={`flex relative h-auto border rounded-lg transition-colors ${
        isFocused
          ? 'border-borderProminent hover:border-borderProminent'
          : 'border-borderSubtle hover:border-borderStandard'
      } bg-bgApp z-10`}
    >
      <textarea
        autoFocus
        id="dynamic-textarea"
        placeholder="What can goose help with?   ⌘↑/⌘↓"
        value={value}
        onChange={handleChange}
        onCompositionStart={handleCompositionStart}
        onCompositionEnd={handleCompositionEnd}
        onKeyDown={handleKeyDown}
        onFocus={() => setIsFocused(true)}
        onBlur={() => setIsFocused(false)}
        ref={textAreaRef}
        rows={1}
        style={{
          minHeight: `${minHeight}px`,
          maxHeight: `${maxHeight}px`,
          overflowY: 'auto',
        }}
        className="w-full pl-4 pr-[68px] outline-none border-none focus:ring-0 bg-transparent py-3 text-sm resize-none text-textStandard"
      />

      <div className="flex items-center justify-center h-full absolute right-2">
        <Button
          type="button"
          size="icon"
          variant="ghost"
          onClick={handleFileSelect}
          className="text-textSubtle rounded-full border border-borderSubtle hover:border-borderStandard hover:text-textStandard w-7 h-7 [&_svg]:size-4"
        >
          <Attach />
        </Button>

        <div className="ml-1">
          {isLoading ? (
            <Button
              type="button"
              size="icon"
              variant="ghost"
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                onStop();
              }}
              className="text-textSubtle rounded-full border border-borderSubtle hover:border-borderProminent hover:text-textStandard w-7 h-7 [&_svg]:size-4"
            >
              <Stop size={24} />
            </Button>
          ) : (
            <Button
              type="submit"
              size="icon"
              variant="ghost"
              disabled={!value.trim()}
              className={`text-textProminentInverse rounded-full border border-borderSubtle bg-bgAppInverse hover:bg-bgStandardInverse w-7 h-7 [&_svg]:size-4 ${
                !value.trim() ? 'text-textProminentInverse cursor-not-allowed' : ''
              }`}
            >
              <Send />
            </Button>
          )}
        </div>
      </div>
    </form>
  );
}
