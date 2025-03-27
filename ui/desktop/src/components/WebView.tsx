import React, { useEffect, useRef, useState } from 'react';

interface WebViewProps {
  url: string;
  isVisible: boolean;
  onClose: () => void;
}

const WebView: React.FC<WebViewProps> = ({ url, isVisible, onClose }) => {
  const webviewRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [width, setWidth] = useState(50); // Width as percentage
  const [isResizing, setIsResizing] = useState(false);

  useEffect(() => {
    // Create the webview element when the component mounts
    if (webviewRef.current) {
      // Clear any existing content
      webviewRef.current.innerHTML = '';

      // Only create the webview if it should be visible
      if (isVisible) {
        setIsLoading(true);
        setLoadError(null);

        // Create a new webview element
        const webview = document.createElement('webview');

        // Set attributes
        webview.src = url;
        webview.style.width = '100%';
        webview.style.height = '100%';
        webview.style.border = 'none';

        // Add webPreferences to disable security features for localhost
        webview.setAttribute('webpreferences', 'contextIsolation=no, nodeIntegration=yes');

        // Add event listener for load completion
        webview.addEventListener('dom-ready', () => {
          console.log('WebView loaded:', url);
          setIsLoading(false);
        });

        // Add error handler
        webview.addEventListener('did-fail-load', (event) => {
          console.error('WebView failed to load:', event);
          setLoadError(`Failed to load: ${event.errorDescription || 'Unknown error'}`);
          setIsLoading(false);
        });

        // Append the webview to our container
        webviewRef.current.appendChild(webview);
      }
    }
  }, [url, isVisible]);

  // Handle resize functionality
  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (isResizing && containerRef.current) {
        // Calculate width as percentage of window width
        const windowWidth = window.innerWidth;
        const newWidth = ((windowWidth - e.clientX) / windowWidth) * 100;

        // Limit width between 30% and 70%
        const clampedWidth = Math.min(Math.max(newWidth, 30), 70);
        setWidth(clampedWidth);
      }
    };

    const handleMouseUp = () => {
      setIsResizing(false);
    };

    if (isResizing) {
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
    }

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isResizing]);

  if (!isVisible) return null;

  return (
    <div
      ref={containerRef}
      className="fixed right-0 top-0 h-full bg-bgApp shadow-xl z-50 flex flex-col animate-slide-in-right"
      style={{ width: `${width}%` }}
    >
      {/* Resize handle */}
      <div
        className="absolute left-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-textStandard hover:opacity-50"
        onMouseDown={() => setIsResizing(true)}
      />

      <div className="flex justify-between items-center p-2 border-b border-borderSubtle">
        <div className="text-sm font-medium text-textStandard overflow-hidden text-ellipsis whitespace-nowrap max-w-[calc(100%-80px)]">
          {url}
        </div>
        <div className="flex items-center">
          <button
            onClick={() => {
              // Force reload the webview
              if (webviewRef.current) {
                const webview = webviewRef.current.querySelector('webview');
                if (webview) {
                  setIsLoading(true);
                  setLoadError(null);
                  (webview as any).reload();
                }
              }
            }}
            className="rounded-full p-1 hover:bg-bgSubtle text-textStandard mr-1"
            title="Reload"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"></path>
              <path d="M21 3v5h-5"></path>
              <path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"></path>
              <path d="M8 16H3v5"></path>
            </svg>
          </button>
          <button
            onClick={onClose}
            className="rounded-full p-1 hover:bg-bgSubtle text-textStandard"
            title="Close"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <line x1="18" y1="6" x2="6" y2="18"></line>
              <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
          </button>
        </div>
      </div>
      <div className="flex-1 relative">
        {isLoading && (
          <div className="absolute inset-0 flex items-center justify-center bg-bgApp bg-opacity-75 z-10">
            <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-textStandard"></div>
          </div>
        )}
        {loadError && (
          <div className="absolute inset-0 flex items-center justify-center bg-bgApp z-10 p-4">
            <div className="bg-red-100 dark:bg-red-900/30 text-red-800 dark:text-red-200 p-4 rounded-md max-w-md">
              <h3 className="font-medium mb-2">Error Loading WebView</h3>
              <p>{loadError}</p>
              <button
                onClick={() => {
                  // Force reload the webview
                  if (webviewRef.current) {
                    webviewRef.current.innerHTML = '';
                    setIsLoading(true);
                    setLoadError(null);

                    // Recreate the webview element
                    const webview = document.createElement('webview');
                    webview.src = url;
                    webview.style.width = '100%';
                    webview.style.height = '100%';
                    webview.style.border = 'none';
                    webview.setAttribute(
                      'webpreferences',
                      'contextIsolation=no, nodeIntegration=yes'
                    );

                    webview.addEventListener('dom-ready', () => {
                      setIsLoading(false);
                    });

                    webview.addEventListener('did-fail-load', (event) => {
                      setLoadError(`Failed to load: ${event.errorDescription || 'Unknown error'}`);
                      setIsLoading(false);
                    });

                    webviewRef.current.appendChild(webview);
                  }
                }}
                className="mt-4 px-4 py-2 bg-red-200 dark:bg-red-800 text-red-800 dark:text-red-200 rounded hover:bg-red-300 dark:hover:bg-red-700 transition-colors"
              >
                Try Again
              </button>
            </div>
          </div>
        )}
        <div ref={webviewRef} className="h-full"></div>
      </div>
    </div>
  );
};

export default WebView;
