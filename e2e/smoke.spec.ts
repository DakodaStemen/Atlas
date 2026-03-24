import { test, expect } from '@playwright/test';

test.describe('Smoke', () => {
  test('homepage loads and has no uncaught console errors', async ({ page }) => {
    const consoleErrors: string[] = [];
    page.on('console', (msg) => {
      const type = msg.type();
      const text = msg.text();
      if (type === 'error') {
        consoleErrors.push(text);
      }
    });

    await page.goto('/');

    expect(consoleErrors, `Console errors: ${consoleErrors.join('; ')}`).toEqual([]);
  });
});
