import React, { useEffect, useRef, useState } from 'react';
import { startCodeServer, stopCodeServer, getWebViewUrl } from '../utils/webViewServer';

interface WebViewProps {
  isVisible: boolean;
  onClose: () => void;
  url?: string;
  onFileSelect?: (fileInfo: { path: string; content: string }) => void;
}

const WebView: React.FC<WebViewProps> = ({ url: initialUrl, isVisible, onClose, onFileSelect }) => {
  const webviewRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [webViewUrl, setWebViewUrl] = useState<string | null>(initialUrl || null);
  const [webviewCreated, setWebviewCreated] = useState(false);

  // Server states
  type ServerState = 'idle' | 'starting' | 'ready' | 'error';
  const [serverState, setServerState] = useState<ServerState>('idle');

  // Start the code server when the component mounts
  useEffect(() => {
    if (webViewUrl || serverState === 'starting' || serverState === 'ready') return;

    const workingDir = window.appConfig.get('GOOSE_WORKING_DIR') || '';

    const startServer = async () => {
      try {
        setServerState('starting');
        setIsLoading(true);

        await startCodeServer(workingDir);
        await new Promise((resolve) => setTimeout(resolve, 2000));
        window.electron.logInfo(`Code server started`);

        const url = await getWebViewUrl(workingDir);
        setWebViewUrl(url);
        setServerState('ready');
      } catch (error) {
        console.error('Failed to start code server:', error);
        setLoadError(`Failed to start code server: ${error.message}`);
        setIsLoading(false);
        setServerState('error');
      }
    };

    startServer();
  }, [webViewUrl, serverState]);

  // Setup IPC communication and VS Code commands
  const setupVSCodeIntegration = (webview: HTMLWebViewElement) => {
    // Listen for messages from VS Code webview
    webview.addEventListener('ipc-message', (event) => {
      console.log('IPC message received:', event.channel, event.args);
      if (event.channel === 'file-selected') {
        const fileInfo = event.args[0];
        console.log('File selected:', fileInfo);
        onFileSelect?.(fileInfo);
      }
    });
    
    // Add console message to verify listener is attached
    console.log('IPC listeners set up for webview');

    // Inject our custom VS Code commands when the webview is ready
    webview.addEventListener('dom-ready', () => {
      const injectCommands = `
        (function() {
          console.log('Attempting to inject Goose commands into VS Code...');
          
          // Wait for VS Code API to be available
          const waitForVSCode = setInterval(() => {
            console.log('Checking for VS Code API...');
            if (typeof acquireVsCodeApi !== 'undefined') {
              console.log('VS Code API found, setting up commands...');
              clearInterval(waitForVSCode);
              const vscode = acquireVsCodeApi();

              // Register command to add current file to Goose context
              vscode.commands.registerCommand('goose.addToContext', () => {
                const editor = vscode.window.activeTextEditor;
                if (editor) {
                  const document = editor.document;
                  window.postMessage({
                    channel: 'file-selected',
                    args: [{
                      path: document.uri.fsPath,
                      content: document.getText()
                    }]
                  });
                }
              });

              // Register command to add selected text to Goose context
              vscode.commands.registerCommand('goose.addSelectionToContext', () => {
                const editor = vscode.window.activeTextEditor;
                if (editor) {
                  const selection = editor.selection;
                  const text = editor.document.getText(selection);
                  const document = editor.document;
                  window.postMessage({
                    channel: 'file-selected',
                    args: [{
                      path: document.uri.fsPath,
                      content: text,
                      isSelection: true
                    }]
                  });
                }
              });

              // Add context menu contributions
              vscode.commands.executeCommand('setContext', 'goose.enabled', true);
            }
          }, 100);
        })();
      `;

      webview.executeJavaScript(injectCommands);
    });
  };

  // Create or update the webview when URL is available and server is ready
  useEffect(() => {
    if (!webviewRef.current || !webViewUrl || serverState !== 'ready') return;

    if (!webviewCreated) {
      console.log('Creating new webview with URL:', webViewUrl);

      webviewRef.current.innerHTML = '';

      const webview = document.createElement('webview');

      webview.src = webViewUrl;
      webview.style.width = '100%';
      webview.style.height = '100%';
      webview.style.border = 'none';

      webview.setAttribute('webpreferences', 'contextIsolation=no, nodeIntegration=yes');

      webview.addEventListener('dom-ready', () => {
        console.log('WebView loaded:', webViewUrl);
        setIsLoading(false);
        setWebviewCreated(true);
        
        // Setup VS Code integration after webview is ready
        setupVSCodeIntegration(webview);
      });

      webview.addEventListener('did-fail-load', (event) => {
        console.error('WebView failed to load:', event);
        setLoadError(`Failed to load: ${event.errorDescription || 'Unknown error'}`);
        setIsLoading(false);
      });

      webviewRef.current.appendChild(webview);
    }
  }, [webViewUrl, webviewCreated, serverState]);

  // Handle visibility changes
  useEffect(() => {
    if (webviewRef.current) {
      const webview = webviewRef.current.querySelector('webview');
      if (webview) {
        console.log(`Setting webview visibility to ${isVisible ? 'visible' : 'hidden'}`);
        if (isVisible) {
          (webview as any).reload();
          setIsLoading(true);
        }
      }
    }
  }, [isVisible]);

  return (
    <div ref={containerRef} className="h-full w-full bg-bgApp flex flex-col">
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
          Code Editor
        </div>
        <div className="flex items-center">
          {/* Test button */}
          <button
            onClick={() => {
              if (webviewRef.current) {
                const webview = webviewRef.current.querySelector('webview');
                if (webview) {
                  console.log('Testing VS Code command...');
                  webview.executeJavaScript(`
                    vscode.commands.executeCommand('goose.addToContext')
                      .then(() => console.log('Command executed'))
                      .catch(err => console.error('Command failed:', err));
                  `);
                }
              }
            }}
            className="rounded-full p-1 hover:bg-bgSubtle text-textStandard mr-1"
            title="Test Command"
          >
            Test
          </button>
          <button
            onClick={() => {
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
                  if (webviewRef.current) {
                    const webview = webviewRef.current.querySelector('webview');
                    if (webview) {
                      setIsLoading(true);
                      setLoadError(null);
                      (webview as any).reload();
                    } else {
                      setWebviewCreated(false);
                    }
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