import { test as base, expect } from '@playwright/test';
import { _electron as electron } from '@playwright/test';
import { join } from 'path';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

// Define provider interface
type Provider = {
  name: string;
};

// Create test fixture type
type TestFixtures = {
  provider: Provider;
};

// Define available providers, keeping as a list of objects for easy expansion
const providers: Provider[] = [
  { name: 'Databricks' },
  { name: 'Google' }
];

// Create test with fixtures
const test = base.extend<TestFixtures>({
  provider: [providers[0], { option: true }], // Default to first provider (Databricks)
});

// Store mainWindow reference
let mainWindow;

test.describe('Goose App Release Tests', () => {
  let electronApp;

  test.beforeAll(async () => {
    console.log('Starting Electron app...');

    // Get electron binary from node_modules
    const electronBinary = require('electron');
    console.log('Using electron binary:', electronBinary);

    // Launch electron with our app
    try {
      console.log('Launching electron app with main.js...');
      electronApp = await electron.launch({
        args: ['.vite/build/main.js'],
        cwd: join(__dirname, '../..'),
        env: {
          GOOSE_ALLOWLIST_BYPASS: 'true',
          GOOSE_TEST_MODE: 'true',
          NODE_ENV: 'development',
          ELECTRON_IS_DEV: '1',
          ELECTRON_ENABLE_LOGGING: '1',
          ELECTRON_ENABLE_STACK_DUMPING: '1',
          DEBUG: '*'
        },
        recordVideo: {
          dir: 'test-results-release/videos/',
          size: { width: 1024, height: 768 }
        }
      });
      console.log('Electron app launched successfully');
    } catch (error) {
      console.error('Failed to launch electron app:', error);
      throw error;
    }

    // Get the main window
    try {
      console.log('Waiting for main window...');
      mainWindow = await electronApp.firstWindow();
      console.log('Main window obtained');

      console.log('Waiting for domcontentloaded...');
      await mainWindow.waitForLoadState('domcontentloaded');
      console.log('domcontentloaded complete');

      console.log('Waiting for networkidle...');
      await mainWindow.waitForLoadState('networkidle');
      console.log('networkidle complete');

      // Take initial screenshot
      await mainWindow.screenshot({ path: 'test-results-release/initial-state.png' });

      // Log current HTML to help debug selectors
      const html = await mainWindow.evaluate(() => document.documentElement.outerHTML);
      console.log('Current page HTML:', html);

      // Ensure window is visible and focused
      await mainWindow.bringToFront();
      
      // Set window bounds to ensure it's visible
      await mainWindow.setViewportSize({ width: 1024, height: 768 });

      // Wait for React app to be ready by looking for any of the known elements
      console.log('Waiting for app elements...');
      await Promise.race([
        mainWindow.waitForSelector('[data-testid="provider-selection-heading"]', { timeout: 30000 }),
        mainWindow.waitForSelector('[data-testid="chat-input"]', { timeout: 30000 }),
        mainWindow.waitForSelector('[data-testid="more-options-button"]', { timeout: 30000 })
      ]);
      console.log('Found app elements');

      // Take another screenshot after waiting
      await mainWindow.screenshot({ path: 'test-results-release/after-wait.png' });
    } catch (error) {
      console.error('Error during window initialization:', error);
      // Take screenshot of the current state if we have a window
      if (mainWindow) {
        await mainWindow.screenshot({ path: 'test-results-release/init-error.png' });
      }
      throw error;
    }
  });

  test.afterAll(async () => {
    console.log('Final cleanup...');

    // Take final screenshot if we have a window
    if (mainWindow) {
      await mainWindow.screenshot({ path: 'test-results-release/final-state.png' }).catch(console.error);
    }

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
  });

  test.describe('General UI', () => {
    test('dark mode toggle', async () => {
      console.log('Testing dark mode toggle...');

      // Take initial screenshot to see what state we're in
      await mainWindow.screenshot({ path: 'test-results-release/initial-test-state.png' });

      try {
        // Select the default provider
        await selectProvider(mainWindow, providers[0]);
      } catch (error) {
        console.error('Error selecting provider:', error);
        // Take error screenshot
        await mainWindow.screenshot({ path: 'test-results-release/provider-select-error.png' });
        throw error;
      }
  
      // Click the three dots menu button in the top right
      const menuButton = await mainWindow.waitForSelector('[data-testid="more-options-button"]', {
        timeout: 5000,
        state: 'visible'
      });
      await menuButton.click();
      await mainWindow.screenshot({ path: 'test-results-release/menu-open.png' });
  
      // Find and click the dark mode toggle button
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
        await mainWindow.screenshot({ path: 'test-results-release/dark-mode-toggle.png' });
      } else {
        // Click to toggle to dark mode
        await darkModeButton.click();
        await mainWindow.waitForTimeout(1000);
        const newDarkMode = await mainWindow.evaluate(() => document.documentElement.classList.contains('dark'));
        expect(newDarkMode).toBe(!isDarkMode);
        await mainWindow.screenshot({ path: 'test-results-release/dark-mode-toggle.png' });
      }

      // check that system mode is clickable
      await systemModeButton.click();
      await mainWindow.screenshot({ path: 'test-results-release/system-mode.png' });
  
      // Toggle back to light mode
      await lightModeButton.click();
      
      // Pause to show return to original state
      await mainWindow.waitForTimeout(2000);
      await mainWindow.screenshot({ path: 'test-results-release/final-mode.png' });
  
      // Close menu with ESC key
      await mainWindow.keyboard.press('Escape');
    });
  });
});

// Helper function to select a provider
async function selectProvider(mainWindow: any, provider: Provider) {
  console.log(`Selecting provider: ${provider.name}`);
  
  // Take screenshot of initial state
  await mainWindow.screenshot({ path: 'test-results-release/provider-select-start.png' });

  // If we're already in the chat interface, we need to reset providers
  console.log('Checking for chat interface...');
  const chatTextarea = await mainWindow.waitForSelector('[data-testid="chat-input"]', { 
    timeout: 5000
  }).catch(() => null);

  if (chatTextarea) {
    console.log('Found chat interface, resetting provider...');
    await mainWindow.screenshot({ path: 'test-results-release/provider-select-chat-found.png' });

    // Click menu button to reset providers
    console.log('Opening menu to reset providers...');
    const menuButton = await mainWindow.waitForSelector('[data-testid="more-options-button"]', {
      timeout: 5000,
      state: 'visible'
    });
    await menuButton.click();

    // Wait for menu to appear and be interactive
    await mainWindow.waitForTimeout(1000);
    await mainWindow.screenshot({ path: 'test-results-release/provider-select-menu-open.png' });

    // Click Reset Provider and Model
    console.log('Clicking Reset provider and model...');
    const resetButton = await mainWindow.waitForSelector('button:has-text("Reset provider and model")', {
      timeout: 5000,
      state: 'visible'
    });
    await resetButton.click();
    await mainWindow.screenshot({ path: 'test-results-release/provider-select-after-reset.png' });
  }

  // Wait for React app to be ready and animations to complete
  console.log('Waiting for provider selection screen...');
  await mainWindow.waitForTimeout(2000);

  // We should now be at provider selection
  const providerHeading = await mainWindow.waitForSelector('[data-testid="provider-selection-heading"]', {
    timeout: 10000
  });
  console.log('Found provider selection heading:', await providerHeading.textContent());

  // Take screenshot before looking for provider card
  await mainWindow.screenshot({ path: 'test-results-release/provider-select-before-card.png' });

  // Find and verify the provider card container
  console.log(`Looking for ${provider.name} card...`);
  let providerContainer;
  try {
    providerContainer = await mainWindow.waitForSelector(`[data-testid="provider-card-${provider.name.toLowerCase()}"]`, {
      timeout: 10000
    });
    expect(await providerContainer.isVisible()).toBe(true);
    console.log('Found provider card');
  } catch (error) {
    console.error(`Provider card not found for ${provider.name}. This could indicate a missing or incorrectly configured provider.`);
    await mainWindow.screenshot({ path: 'test-results-release/provider-select-card-error.png' });
    throw error;
  }

  // Find the Launch button within the provider container
  console.log(`Looking for Launch button in ${provider.name} card...`);
  const launchButton = await providerContainer.waitForSelector('[data-testid="provider-launch-button"]', {
    timeout: 10000
  });
  expect(await launchButton.isVisible()).toBe(true);
  console.log('Found launch button');

  // Take screenshot before clicking
  await mainWindow.screenshot({ path: 'test-results-release/provider-select-before-launch.png' });

  // Click the Launch button
  await launchButton.click();
  console.log('Clicked launch button');

  // Wait for chat interface to appear
  console.log('Waiting for chat interface...');
  const chatTextareaAfterClick = await mainWindow.waitForSelector('[data-testid="chat-input"]', {
    timeout: 10000
  });
  expect(await chatTextareaAfterClick.isVisible()).toBe(true);
  console.log('Found chat interface');

  // Take screenshot of chat interface
  await mainWindow.screenshot({ path: 'test-results-release/provider-select-complete.png' });
}