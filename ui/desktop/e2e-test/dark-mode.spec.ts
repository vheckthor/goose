import { test, expect } from '@playwright/test';
import { _electron as electron } from '@playwright/test';
import { join } from 'path';
import { spawn, exec } from 'child_process';
import { promisify } from 'util';
import { showTestName, clearTestName } from './test-overlay';

const execAsync = promisify(exec);

let mainWindow;

test.describe('Goose App Dark Mode', () => {
  let electronApp;
  let appProcess;

  test.beforeAll(async () => {
    console.log('Starting Electron app...');

    // Start the electron-forge process
    appProcess = spawn('npm', ['run', 'start-gui'], {
      cwd: join(__dirname, '..'),
      stdio: 'pipe',
      shell: true,
      env: {
        ...process.env,
        ELECTRON_IS_DEV: '1',
        NODE_ENV: 'development',
        GOOSE_ALLOWLIST_BYPASS: 'true',
      }
    });

    // Log process output
    appProcess.stdout.on('data', (data) => {
      console.log('App stdout:', data.toString());
    });

    appProcess.stderr.on('data', (data) => {
      console.log('App stderr:', data.toString());
    });

    // Wait a bit for the app to start
    console.log('Waiting for app to start...');
    await new Promise(resolve => setTimeout(resolve, 5000));

    // Launch Electron for testing
    electronApp = await electron.launch({
      args: ['.vite/build/main.js'],
      cwd: join(__dirname, '..'),
      env: {
        ...process.env,
        ELECTRON_IS_DEV: '1',
        NODE_ENV: 'development',
        // Suppress XPC connection warnings
        NSDocumentRevisionsDebugMode: 'YES',
      },
      recordVideo: {
        dir: 'test-results/videos/',
        size: { width: 620, height: 680 }
      }
    });

    // Get the main window
    mainWindow = await electronApp.firstWindow();
    await mainWindow.waitForLoadState('domcontentloaded');
    await mainWindow.waitForLoadState('networkidle');

    // Wait for React app to be ready
    await mainWindow.waitForFunction(() => {
      const root = document.getElementById('root');
      return root && root.children.length > 0;
    });

    // Wait for any animations to complete
    await mainWindow.waitForTimeout(2000);

    // Take a screenshot to debug what's on screen
    await mainWindow.screenshot({ path: 'test-results/initial-load.png' });
  });

  test.afterAll(async () => {
    console.log('Final cleanup...');

    // Close the test instance
    if (electronApp) {
      await electronApp.close().catch(console.error);
    }

    // Kill any remaining electron processes
    try {
      if (process.platform === 'win32') {
        await execAsync('taskkill /F /IM electron.exe');
      } else {
        await execAsync('pkill -f electron || true');
      }
    } catch (error) {
      if (!error.message?.includes('no process found')) {
        console.error('Error killing electron processes:', error);
      }
    }

    // Kill any remaining npm processes from start-gui
    try {
      if (process.platform === 'win32') {
        await execAsync('taskkill /F /IM node.exe');
      } else {
        await execAsync('pkill -f "start-gui" || true');
      }
    } catch (error) {
      if (!error.message?.includes('no process found')) {
        console.error('Error killing npm processes:', error);
      }
    }

    // Kill the specific npm process if it's still running
    try {
      if (appProcess && appProcess.pid) {
        process.kill(appProcess.pid);
      }
    } catch (error) {
      if (error.code !== 'ESRCH') {
        console.error('Error killing npm process:', error);
      }
    }
  });

  test('dark mode toggle', async () => {
    console.log('Testing dark mode toggle...');

    // Add test name overlay
    await showTestName(mainWindow, 'dark mode toggle');

    // Wait for more options button to be visible
    const menuButton = await mainWindow.waitForSelector('[data-testid="more-options-button"]', {
      timeout: 30000,
      state: 'visible'
    });

    // Take screenshot before clicking menu
    await mainWindow.screenshot({ path: 'test-results/before-menu.png' });

    // Click the menu button
    await menuButton.click();

    // Wait for theme options to appear
    const darkModeButton = await mainWindow.waitForSelector('[data-testid="dark-mode-button"]');
    const lightModeButton = await mainWindow.waitForSelector('[data-testid="light-mode-button"]');
    const systemModeButton = await mainWindow.waitForSelector('[data-testid="system-mode-button"]');

    // Get initial state
    const isDarkMode = await mainWindow.evaluate(() => document.documentElement.classList.contains('dark'));
    console.log('Initial dark mode state:', isDarkMode);

    if (isDarkMode) {
      // Click to toggle to light mode
      await lightModeButton.click();
      await mainWindow.waitForTimeout(1000);
      const newDarkMode = await mainWindow.evaluate(() => document.documentElement.classList.contains('dark'));
      expect(newDarkMode).toBe(!isDarkMode);
      // Take screenshot to verify and pause to show the change
      await mainWindow.screenshot({ path: 'test-results/dark-mode-toggle.png' });
    } else {
      // Click to toggle to dark mode
      await darkModeButton.click();
      await mainWindow.waitForTimeout(1000);
      const newDarkMode = await mainWindow.evaluate(() => document.documentElement.classList.contains('dark'));
      expect(newDarkMode).toBe(!isDarkMode);
    }

    // check that system mode is clickable
    await systemModeButton.click();

    // Toggle back to light mode
    await lightModeButton.click();
    
    // Pause to show return to original state
    await mainWindow.waitForTimeout(2000);

    // Close menu with ESC key
    await mainWindow.keyboard.press('Escape');

    // Clear test name overlay
    await clearTestName(mainWindow);
  });
});