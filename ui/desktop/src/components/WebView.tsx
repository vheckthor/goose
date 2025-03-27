import React, { useEffect, useRef, useState } from 'react';
import { startCodeServer, stopCodeServer, getWebViewUrl } from '../utils/webViewServer';

interface WebViewProps {
  isVisible: boolean;
  onClose: () => void;
  url?: string; // Make url optional as we'll generate it dynamically
}

const WebView: React.FC<WebViewProps> = ({ url: initialUrl, isVisible, onClose }) => {
  const webviewRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [isResizing, setIsResizing] = useState(false);
  const [webViewUrl, setWebViewUrl] = useState<string | null>(initialUrl || null);

  useEffect(() => {
    // Start the code server when the component becomes visible
    if (isVisible && !webViewUrl) {
      const workingDir = window.appConfig.get('GOOSE_WORKING_DIR') || '';

      // Start the code server and get the URL
      const startServer = async () => {
        try {
          setIsLoading(true);
          await startCodeServer(workingDir);
          const url = await getWebViewUrl(workingDir);
          // sleep for a few seconds
          await new Promise((resolve) => setTimeout(resolve, 2000));
          setWebViewUrl(url);
        } catch (error) {
          console.error('Failed to start code server:', error);
          setLoadError(`Failed to start code server: ${error.message}`);
          setIsLoading(false);
        }
      };

      startServer();
    }

    // Note: We no longer stop the server when the component unmounts
    // The server will continue running until the app exits
    // This is handled by the main process in app.on('will-quit')
    return () => {
      // Just clear the URL state when the component unmounts
      if (!isVisible) {
        setWebViewUrl(null);
      }
    };
  }, [isVisible, webViewUrl]);

  useEffect(() => {
    // Create the webview element when the component mounts
    if (webviewRef.current) {
      // Clear any existing content
      webviewRef.current.innerHTML = '';

      // Only create the webview if it should be visible and we have a URL
      if (isVisible && webViewUrl) {
        setIsLoading(true);
        setLoadError(null);

        // Create a new webview element
        const webview = document.createElement('webview');

        // Set attributes
        webview.src = webViewUrl;
        webview.style.width = '100%';
        webview.style.height = '100%';
        webview.style.border = 'none';

        // Add webPreferences to disable security features for localhost
        webview.setAttribute('webpreferences', 'contextIsolation=no, nodeIntegration=yes');

        // Add event listener for load completion
        webview.addEventListener('dom-ready', () => {
          console.log('WebView loaded:', webViewUrl);
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
  }, [webViewUrl, isVisible]);

  // Handle resize functionality
  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (isResizing && containerRef.current) {
        // Calculate width as percentage of window width
        const windowWidth = window.innerWidth;
        const newWidth = ((windowWidth - e.clientX) / windowWidth) * 100;

        // Limit width between 30% and 80%
        const clampedWidth = Math.min(Math.max(newWidth, 30), 80);

        // Update WebView width
        containerRef.current.style.width = `${clampedWidth}%`;
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
      style={{ width: '66.67%' }}
    >
      {/* Resize handle */}
      <div
        className="absolute left-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-textStandard hover:opacity-50"
        onMouseDown={() => setIsResizing(true)}
      />

      <div className="flex justify-between items-center p-2 border-b border-borderSubtle">
        <div className="text-sm font-medium text-textStandard overflow-hidden text-ellipsis whitespace-nowrap max-w-[calc(100%-80px)] flex items-center">
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
            className="mr-2"
          >
            <rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect>
            <line x1="8" y1="21" x2="16" y2="21"></line>
            <line x1="12" y1="17" x2="12" y2="21"></line>
          </svg>
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
