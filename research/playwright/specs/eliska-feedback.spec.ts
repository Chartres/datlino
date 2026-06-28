import { test, expect } from '@playwright/test';
import { defaultCorpus, installMockTauri } from '../fixtures/mock-tauri';

/**
 * Eliška's first-run journey + the new feedback loop, with a screenshot at every
 * meaningful state (Flywheel Standard: persona testing is visual). The shots are
 * for human/agent inspection, not just green assertions.
 */
const SHOTS = 'research/playwright/screenshots';

test.describe('Eliška — first-run + feedback loop (visual)', () => {
  test.beforeEach(async ({ page }) => {
    await installMockTauri(page, defaultCorpus({ embeddingProvider: 'fake', sessionsCompleted: 0 }));
  });

  test('first-run doors → learn → privacy consent → submit feedback', async ({ page }) => {
    // 1. First run: the two doors
    await page.goto('/');
    await page.waitForSelector('header h1');
    await page.screenshot({ path: `${SHOTS}/fb-1-home.png`, fullPage: true });
    await expect(page.locator('a.door.learn')).toBeVisible();

    // 2. The beginner door → Learn ladder
    await page.click('a.door.learn');
    await page.waitForLoadState('networkidle');
    await page.screenshot({ path: `${SHOTS}/fb-2-learn.png`, fullPage: true });

    // 3. Settings → Soukromí: analytics opt-in (off by default) + feedback
    await page.goto('/settings');
    await page.getByText('Soukromí', { exact: false }).first().click();
    const consent = page.locator('.consent input[type="checkbox"]');
    await expect(consent).not.toBeChecked(); // privacy promise: off until opted in
    await page.screenshot({ path: `${SHOTS}/fb-3-privacy-default-off.png`, fullPage: true });

    // 4. Opt in
    await consent.check();
    await expect(consent).toBeChecked();
    await page.screenshot({ path: `${SHOTS}/fb-4-consent-on.png`, fullPage: true });

    // 5. Open + fill the feedback widget
    await page.getByRole('button', { name: /Napiš nám/ }).first().click();
    await page.getByRole('button', { name: /Hodně by mi chybělo/ }).click();
    await page.locator('.fb textarea').first().fill('Diakritika je super, chtělo by to víc lekcí.');
    await page.screenshot({ path: `${SHOTS}/fb-5-feedback-filled.png`, fullPage: true });

    // 6. Submit → thank-you state (ships dark in test, UI still confirms)
    await page.getByRole('button', { name: 'Odeslat' }).first().click();
    await expect(page.getByText(/Díky/).first()).toBeVisible();
    await page.screenshot({ path: `${SHOTS}/fb-6-feedback-sent.png`, fullPage: true });
  });
});
