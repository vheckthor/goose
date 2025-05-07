import React, { createContext, useContext, useState } from 'react';
import { Button } from './ui/button';
import { Maximize2, Minimize2 } from 'lucide-react';

// Create a context to store the compact mode state
interface ToolCallViewContextType {
  isCompactMode: boolean;
  toggleCompactMode: () => void;
}

const ToolCallViewContext = createContext<ToolCallViewContextType>({
  isCompactMode: true,
  toggleCompactMode: () => {},
});

// Hook to use the compact mode context
export const useToolCallViewMode = () => useContext(ToolCallViewContext);

// Provider component to wrap around the application
export function ToolCallViewProvider({ children }: { children: React.ReactNode }) {
  const [isCompactMode, setIsCompactMode] = useState<boolean>(true);

  const toggleCompactMode = () => {
    setIsCompactMode((prev) => !prev);
  };

  return (
    <ToolCallViewContext.Provider value={{ isCompactMode, toggleCompactMode }}>
      {children}
    </ToolCallViewContext.Provider>
  );
}

// Toggle button component
export function ToolCallViewToggle() {
  const { isCompactMode, toggleCompactMode } = useToolCallViewMode();

  return (
    <Button
      variant="ghost"
      size="sm"
      onClick={toggleCompactMode}
      className="flex items-center gap-1 text-xs"
      title={isCompactMode ? 'Switch to expanded tool view' : 'Switch to compact tool view'}
    >
      {isCompactMode ? (
        <>
          <Maximize2 className="h-3 w-3" />
          <span className="hidden sm:inline">Expand Tools</span>
        </>
      ) : (
        <>
          <Minimize2 className="h-3 w-3" />
          <span className="hidden sm:inline">Compact Tools</span>
        </>
      )}
    </Button>
  );
}
