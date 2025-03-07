import { Popover, PopoverContent, PopoverPortal, PopoverTrigger } from '../components/ui/popover';
import React, { useEffect, useState } from 'react';
import { ChatSmart, Idea, Refresh, Settings, Time } from './icons';
import { FolderOpen, Moon, Sliders, Sun } from 'lucide-react';
import { View } from '../App';

interface VersionInfo {
  current_version: string;
  available_versions: string[];
}

interface MenuButtonProps {
  onClick: () => void;
  children: React.ReactNode;
  subtitle?: string;
  className?: string;
  danger?: boolean;
}

const MenuButton: React.FC<MenuButtonProps> = ({
  onClick,
  children,
  subtitle,
  className = '',
  danger = false,
}) => (
  <button
    onClick={onClick}
    className={`w-full text-left px-4 py-3 min-h-[64px] text-sm hover:bg-bgSubtle transition-colors border-b border-borderSubtle ${
      danger ? 'text-red-400' : ''
    } ${className}`}
  >
    <div className="flex flex-col">
      <span>{children}</span>
      {subtitle && <span className="text-xs text-textSubtle mt-0.5">{subtitle}</span>}
    </div>
  </button>
);

interface DarkModeToggleProps {
  isDarkMode: boolean;
  onToggle: () => void;
}

const DarkModeToggle: React.FC<DarkModeToggleProps> = ({ isDarkMode, onToggle }) => (
  <button
    className="flex items-center min-h-[64px] justify-between px-4 py-3 hover:bg-bgSubtle border-b border-borderSubtle transition-colors"
    onClick={onToggle}
  >
    <span className="text-sm">{isDarkMode ? 'Light Mode' : 'Dark Mode'}</span>
    <div className="h-5 w-5 overflow-hidden relative rounded-full">
      <div className="absolute right-[-1px] bg-bg flex h-5 w-5 flex-row items-center justify-center transition-all rotate-180 dark:rotate-0 translate-x-[100%] dark:translate-x-[0%]">
        <svg
          width="24"
          height="24"
          viewBox="0 0 24 24"
          fill="currentColor"
          xmlns="http://www.w3.org/2000/svg"
          className="h-5 w-5 text-[#fac64d] transition-all duration-[400ms]"
        >
          <path d="M6.995 12C6.995 14.761 9.241 17.007 12.002 17.007C14.763 17.007 17.009 14.761 17.009 12C17.009 9.239 14.763 6.993 12.002 6.993C9.241 6.993 6.995 9.239 6.995 12ZM11 19H13V22H11V19ZM11 2H13V5H11V2ZM2 11H5V13H2V11ZM19 11H22V13H19V11Z"></path>
          <path d="M5.63702 19.778L4.22302 18.364L6.34402 16.243L7.75802 17.657L5.63702 19.778Z"></path>
          <path d="M16.242 6.34405L18.364 4.22205L19.778 5.63605L17.656 7.75805L16.242 6.34405Z"></path>
          <path d="M6.34402 7.75902L4.22302 5.63702L5.63802 4.22302L7.75802 6.34502L6.34402 7.75902Z"></path>
          <path d="M19.778 18.3639L18.364 19.7779L16.242 17.6559L17.656 16.2419L19.778 18.3639Z"></path>
        </svg>
      </div>

      <div className="absolute right-[-1px] bg-bg flex h-5 w-5 flex-row items-center justify-center transition-all dark:translate-x-[-100%] dark:-rotate-90">
        <svg
          width="24"
          height="24"
          viewBox="0 0 24 24"
          fill="currentColor"
          xmlns="http://www.w3.org/2000/svg"
          className="h-5 w-5 text-[#8b8bf8] transition-all duration-[400ms]"
        >
          <path d="M12 11.807C10.7418 10.5483 9.88488 8.94484 9.53762 7.1993C9.19037 5.45375 9.36832 3.64444 10.049 2C8.10826 2.38205 6.3256 3.33431 4.92899 4.735C1.02399 8.64 1.02399 14.972 4.92899 18.877C8.83499 22.783 15.166 22.782 19.072 18.877C20.4723 17.4805 21.4245 15.6983 21.807 13.758C20.1625 14.4385 18.3533 14.6164 16.6077 14.2692C14.8622 13.9219 13.2588 13.0651 12 11.807V11.807Z"></path>
        </svg>
      </div>
    </div>
  </button>
);

export default function MoreMenu({
  setView,
  setIsGoosehintsModalOpen,
}: {
  setView: (view: View) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
}) {
  const [open, setOpen] = useState(false);
  const [versions, setVersions] = useState<VersionInfo | null>(null);
  const [showVersions, setShowVersions] = useState(false);
  const [useSystemTheme, setUseSystemTheme] = useState(
    () => localStorage.getItem('use_system_theme') === 'true'
  );
  const [isDarkMode, setDarkMode] = useState(() => {
    const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    if (useSystemTheme) {
      return systemPrefersDark;
    }
    const savedTheme = localStorage.getItem('theme');
    return savedTheme ? savedTheme === 'dark' : systemPrefersDark;
  });

  useEffect(() => {
    const fetchVersions = async () => {
      try {
        const port = window.appConfig.get('GOOSE_PORT');
        const response = await fetch(`http://127.0.0.1:${port}/agent/versions`);
        if (!response.ok) {
          throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        setVersions(data);
      } catch (error) {
        console.error('Failed to fetch versions:', error);
      }
    };

    if (open) {
      fetchVersions();
    }
  }, [open]);

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleThemeChange = (e: { matches: boolean }) => {
      if (useSystemTheme) {
        setDarkMode(e.matches);
      }
    };

    mediaQuery.addEventListener('change', handleThemeChange);
    if (useSystemTheme) {
      setDarkMode(mediaQuery.matches);
    } else {
      const savedTheme = localStorage.getItem('theme');
      setDarkMode(savedTheme ? savedTheme === 'dark' : mediaQuery.matches);
    }

    return () => mediaQuery.removeEventListener('change', handleThemeChange);
  }, [useSystemTheme]);

  useEffect(() => {
    if (isDarkMode) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
    if (!useSystemTheme) {
      localStorage.setItem('theme', isDarkMode ? 'dark' : 'light');
    }
  }, [isDarkMode, useSystemTheme]);

  const toggleTheme = () => {
    if (!useSystemTheme) {
      setDarkMode(!isDarkMode);
    }
  };

  const handleVersionSelect = (version: string) => {
    setOpen(false);
    setShowVersions(false);
    window.electron.createChatWindow(undefined, undefined, version);
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button
          className={`z-[100] absolute top-2 right-4 w-[20px] h-[20px] transition-colors cursor-pointer no-drag hover:text-textProminent ${open ? 'text-textProminent' : 'text-textSubtle'}`}
        >
          <Settings />
        </button>
      </PopoverTrigger>

      <PopoverPortal>
        <PopoverContent
          className="z-[200] w-[375px] mt-2 -mr-2 rounded-lg bg-bgApp border border-borderSubtle text-textStandard"
          align="end"
          sideOffset={5}
        >
          <div className="flex flex-col rounded-md">
            <DarkModeToggle isDarkMode={isDarkMode} onToggle={toggleTheme} />

            <MenuButton
              onClick={() => setIsGoosehintsModalOpen(true)}
              subtitle="Customize AI behavior for this directory"
            >
              Configure .goosehints
            </MenuButton>

            <MenuButton
              onClick={() => {
                setOpen(false);
                window.electron.directoryChooser();
              }}
              subtitle="Open a new directory to work with"
            >
              Open Directory
              <span className="text-textSubtle ml-1">(cmd+O)</span>
            </MenuButton>

            <MenuButton
              onClick={() => {
                setOpen(false);
                window.electron.createChatWindow();
              }}
              subtitle="Start a new chat session"
            >
              New Session
              <span className="text-textSubtle ml-1">(cmd+N)</span>
            </MenuButton>

            <MenuButton
              onClick={() => setView('sessions')}
              subtitle="View previous goose sessions and their contents"
            >
              Session history
            </MenuButton>

            <MenuButton
              onClick={() => {
                setOpen(false);
                setView('settings');
              }}
              subtitle="Configure application settings"
            >
              Settings
              <span className="text-textSubtle ml-1">(cmd+,)</span>
            </MenuButton>

            <MenuButton
              onClick={() => {
                localStorage.removeItem('GOOSE_PROVIDER');
                setOpen(false);
                window.electron.createChatWindow();
              }}
              danger
              subtitle="Clear selected AI provider and restart"
            >
              Reset Provider
            </MenuButton>

            {process.env.ALPHA && (
              <MenuButton
                onClick={() => {
                  setOpen(false);
                  window.electron.createChatWindow(
                    undefined,
                    window.appConfig.get('GOOSE_WORKING_DIR')
                  );
                }}
                className="text-indigo-800"
                subtitle="Preview the new provider configuration interface"
              >
                See new providers grid
              </MenuButton>
            )}
          </div>
        </PopoverContent>
      </PopoverPortal>
    </Popover>
  );
}
