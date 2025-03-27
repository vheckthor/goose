/**
 * Utility functions for managing the code server for WebView
 */

/**
 * Starts the code server process
 * The server will continue running until the app exits
 */
export async function startCodeServer(
  workingDir: string
): Promise<{ port: number; token: string }> {
  return window.electron.startCodeServer(workingDir);
}

/**
 * Stops the code server process if it's running
 * Note: This is typically not needed as the server is stopped when the app exits
 */
export function stopCodeServer(): Promise<boolean> {
  return window.electron.stopCodeServer();
}

/**
 * Gets the URL for the webview
 */
export function getWebViewUrl(workingDir: string): Promise<string | null> {
  return window.electron.getWebViewUrl(workingDir);
}
