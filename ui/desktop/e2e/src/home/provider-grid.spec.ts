import { ElectronApplication, expect, test } from '@playwright/test';
import { startApp } from '../../test-utils/start-app';

let electronApp: ElectronApplication;
test.beforeEach(async () => {
  electronApp = await startApp();
});

test.afterEach(async () => {
  await electronApp.close();
});

test('Home - Provider Grid', async () => {
  const window = await electronApp.firstWindow();
  await expect(window.locator('h1', { hasText: 'Welcome to Goose' })).toBeVisible();
  await expect(window).toHaveScreenshot('home-provider-grid.png');
});

test('Home - Provider Grid - Setup', async () => {
  const window = await electronApp.firstWindow();
  window.route('**/configs/store', (route) => {
    route.fulfill({ status: 200, body: JSON.stringify({ error: false }) });
  });
  await expect(window.locator('h1', { hasText: 'Welcome to Goose' })).toBeVisible();
  await window.getByTestId('Databricks-provider-card-setup-button').click();
  await expect(window.getByText('Setup Databricks')).toBeVisible();
  await expect(window).toHaveScreenshot('home-provider-grid-setup.png');
  await window.getByPlaceholder('DATABRICKS_HOST').fill('https://databricks.com');
  await window.getByText('Submit').click();
  await expect(window.locator('div[role=alert]', { hasText: 'Successfully added' })).toBeVisible();
});
