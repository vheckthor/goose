import React, { useRef, useState, useEffect } from 'react';
import { Button } from './ui/button';
import Stop from './ui/Stop';
import type { View } from '../App';
import { Attach, ChevronDown, Send, Document, ChevronUp } from './icons';
import { Sliders } from 'lucide-react';
import { useModel } from './settings/models/ModelContext';
import { ModelRadioList } from './settings/models/ModelRadioList';
import { settingsV2Enabled } from '../flags';
import { BottomMenuModeSelection } from './BottomMenuModeSelection';
import ModelsBottomBar from './settings_v2/models/bottom_bar/ModelsBottomBar';
import ToolCount from './ToolCount';

interface ChatBarProps {
  handleSubmit: (e: React.FormEvent) => void;
  isLoading?: boolean;
  onStop?: () => void;
  commandHistory?: string[];
  initialValue?: string;
  setView: (view: View) => void;
  hasMessages?: boolean;
}

export default function ChatBar({
  handleSubmit,
  isLoading = false,
  onStop,
  commandHistory = [],
  initialValue = '',
  setView,
  hasMessages = false,
}: ChatBarProps) {
  // Input state
  const [value, setValue] = useState(initialValue);
  const [isFocused, setIsFocused] = useState(false);
  const [isComposing, setIsComposing] = useState(false);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [savedInput, setSavedInput] = useState('');
  const textAreaRef = useRef<HTMLTextAreaElement>(null);

  // Model menu state
  const [isModelMenuOpen, setIsModelMenuOpen] = useState(false);
  const { currentModel } = useModel();
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Constants
  const minHeight = '1rem';
  const maxHeight = 10 * 24;

  // Update internal value when initialValue changes
  useEffect(() => {
    if (initialValue) {
      setValue(initialValue);
    }
  }, [initialValue]);

  // Focus textarea on mount
  useEffect(() => {
    if (textAreaRef.current) {
      textAreaRef.current.focus();
    }
  }, []);

  // Handle clicks outside model menu
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsModelMenuOpen(false);
      }
    };

    if (isModelMenuOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isModelMenuOpen]);

  // Handle Escape key for model menu
  useEffect(() => {
    const handleEsc = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setIsModelMenuOpen(false);
      }
    };

    if (isModelMenuOpen) {
      window.addEventListener('keydown', handleEsc);
    }

    return () => {
      window.removeEventListener('keydown', handleEsc);
    };
  }, [isModelMenuOpen]);

  // Textarea autosize
  const useAutosizeTextArea = (textAreaRef: HTMLTextAreaElement | null, value: string) => {
    useEffect(() => {
      if (textAreaRef) {
        textAreaRef.style.height = '0px'; // Reset height
        const scrollHeight = textAreaRef.scrollHeight;
        textAreaRef.style.height = Math.min(scrollHeight, maxHeight) + 'px';
      }
    }, [textAreaRef, value]);
  };

  useAutosizeTextArea(textAreaRef.current, value);

  const handleChange = (evt: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = evt.target.value;
    setValue(val);
  };

  const handleCompositionStart = () => {
    setIsComposing(true);
  };

  const handleCompositionEnd = () => {
    setIsComposing(false);
  };

  const handleHistoryNavigation = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    evt.preventDefault();

    if (historyIndex === -1) {
      setSavedInput(value);
    }

    let newIndex = historyIndex;
    if (evt.key === 'ArrowUp') {
      if (historyIndex < commandHistory.length - 1) {
        newIndex = historyIndex + 1;
      }
    } else {
      if (historyIndex > -1) {
        newIndex = historyIndex - 1;
      }
    }

    if (newIndex == historyIndex) {
      return;
    }

    setHistoryIndex(newIndex);
    if (newIndex === -1) {
      setValue(savedInput);
    } else {
      setValue(commandHistory[newIndex] || '');
    }
  };

  const handleKeyDown = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if ((evt.metaKey || evt.ctrlKey) && (evt.key === 'ArrowUp' || evt.key === 'ArrowDown')) {
      handleHistoryNavigation(evt);
      return;
    }

    if (evt.key === 'Enter') {
      if (evt.shiftKey || isComposing) {
        return;
      }
      if (evt.altKey) {
        setValue(value + '\n');
        return;
      }

      evt.preventDefault();

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
      setValue((prev) => {
        const currentText = prev.trim();
        return currentText ? `${currentText} ${path}` : path;
      });
      textAreaRef.current?.focus();
    }
  };

  return (
    <div
      className={`flex flex-col relative h-auto border rounded-lg transition-colors ${
        isFocused
          ? 'border-borderProminent hover:border-borderProminent'
          : 'border-borderSubtle hover:border-borderStandard'
      } bg-bgApp z-10`}
    >
      {/* Input Form */}
      <form onSubmit={onFormSubmit}>
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

        <div className="flex items-center justify-center absolute right-2 top-2">
          <Button
            type="button"
            size="icon"
            variant="ghost"
            onClick={handleFileSelect}
            className="text-textSubtle rounded-full border border-borderSubtle hover:border-borderStandard hover:text-textStandard w-7 h-7 [&_svg]:size-4"
          >
            <Attach />
          </Button>
        </div>
      </form>

      {/* Bottom Menu */}
      <div className="flex justify-end items-center justify-between transition-colors text-textSubtle relative text-xs p-2 border-t border-borderSubtle">
        {/* Directory Chooser */}
        {/* <span
          className="hover:cursor-pointer hover:text-textStandard flex items-center [&>svg]:size-4"
          onClick={async () => {
            if (hasMessages) {
              window.electron.directoryChooser();
            } else {
              window.electron.directoryChooser(true);
            }
          }}
        >
          <Document className="mr-1" />
          <div className="w-max-[200px] truncate [direction:rtl]">
            Working in {window.appConfig.get('GOOSE_WORKING_DIR')}
          </div>
        </span> */}

        <div className="flex items-center gap-2">
          <Button
            type="button"
            size="icon"
            variant="ghost"
            onClick={handleFileSelect}
            className="text-textSubtle rounded-full border border-borderSubtle hover:border-borderStandard hover:text-textStandard w-7 h-7 [&_svg]:size-4"
          >
            <Attach />
          </Button>

          {/* Mode Selector */}
          <BottomMenuModeSelection />

          <div className="w-[1px] h-4 bg-borderSubtle" />

          {/* Model Selector */}
          {settingsV2Enabled ? (
            <ModelsBottomBar dropdownRef={dropdownRef} setView={setView} />
          ) : (
            <div className="relative flex items-center ml-0 mr-4" ref={dropdownRef}>
              <div
                className="flex items-center cursor-pointer"
                onClick={() => setIsModelMenuOpen(!isModelMenuOpen)}
              >
                <span>{(currentModel?.alias ?? currentModel?.name) || 'Select Model'}</span>
                {isModelMenuOpen ? (
                  <ChevronDown className="w-4 h-4 ml-1" />
                ) : (
                  <ChevronUp className="w-4 h-4 ml-1" />
                )}
              </div>

              {/* Model Dropdown Menu */}
              {isModelMenuOpen && (
                <div className="absolute bottom-[24px] right-0 w-[300px] bg-bgApp rounded-lg border border-borderSubtle">
                  <div className="">
                    <ModelRadioList
                      className="divide-y divide-borderSubtle"
                      renderItem={({ model, isSelected, onSelect }) => (
                        <label key={model.alias ?? model.name} className="block cursor-pointer">
                          <div
                            className="flex items-center justify-between p-2 text-textStandard hover:bg-bgSubtle transition-colors"
                            onClick={onSelect}
                          >
                            <div>
                              <p className="text-sm ">{model.alias ?? model.name}</p>
                              <p className="text-xs text-textSubtle">
                                {model.subtext ?? model.provider}
                              </p>
                            </div>
                            <div className="relative">
                              <input
                                type="radio"
                                name="recentModels"
                                value={model.name}
                                checked={isSelected}
                                onChange={onSelect}
                                className="peer sr-only"
                              />
                              <div
                                className="h-4 w-4 rounded-full border border-gray-400 dark:border-gray-500
                                peer-checked:border-[6px] peer-checked:border-black dark:peer-checked:border-white
                                peer-checked:bg-white dark:peer-checked:bg-black
                                transition-all duration-200 ease-in-out"
                              ></div>
                            </div>
                          </div>
                        </label>
                      )}
                    />
                    <div
                      className="flex items-center justify-between text-textStandard p-2 cursor-pointer hover:bg-bgStandard
                      border-t border-borderSubtle mt-2"
                      onClick={() => {
                        setIsModelMenuOpen(false);
                        setView('settings');
                      }}
                    >
                      <span className="text-sm">Tools and Settings</span>
                      <Sliders className="w-5 h-5 ml-2 rotate-90" />
                    </div>
                  </div>
                </div>
              )}
            </div>
          )}

          {/* Tool count */}
          <ToolCount />
        </div>

        {/* Right-side section */}
        <div className="flex items-center">
          <div className="w-[1px] h-4 bg-borderSubtle mx-2" />
        </div>
      </div>
    </div>
  );
}
