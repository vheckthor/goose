import type { PlaywrightTestConfig } from '@playwright/test';

const config: PlaywrightTestConfig = {
  testDir: './e2e-test',
  timeout: 60000,
  expect: {
    timeout: 10000,
  },
  fullyParallel: false,
  forbidOnly: true, // Always fail if test.only is present
  retries: 2, // Retry failed tests twice
  workers: 1,
  reporter: 'github',
  use: {
    actionTimeout: 0,
    trace: 'retain-on-failure',
    video: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  outputDir: 'test-results',
  testMatch: '**/*.spec.ts',
  // CI-specific options
  projects: [
    {
      name: 'headless',
      use: {
        // Force headless mode
        launchOptions: {
          headless: true,
        },
      },
    },
  ],
};

export default config;