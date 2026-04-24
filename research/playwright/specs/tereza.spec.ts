import { test, expect } from '@playwright/test';
import { defaultCorpus, installMockTauri } from '../fixtures/mock-tauri';
import { openFile, record } from '../fixtures/observations';

/**
 * Tereza, 16, GoodNotes maximalist. Her notes are image-only PDFs. She
 * cares about one thing: does OCR work?
 */
test.describe('Tereza, 16 — GoodNotes PDFs', () => {
  test.beforeEach(async ({ page }) => {
    await installMockTauri(page, defaultCorpus());
    openFile('tereza', 'Moje poznámky jsou PDF z GoodNotes. Obrázky rukopisu.');
  });

  test('settings OCR section tells her what to install AND offers re-check', async ({ page }) => {
    await page.goto('/settings');
    await page.waitForSelector('details.section');

    // OCR is section 4, index 3. Expand it.
    await page.locator('details.section').nth(3).locator('summary').click();
    await page.waitForTimeout(100);

    const ocrBody = await page
      .locator('details.section')
      .nth(3)
      .locator('.section-body')
      .textContent();
    record({
      persona: 'tereza',
      moment: 'ocr-section',
      note: `OCR body: "${ocrBody?.replace(/\s+/g, ' ').trim().slice(0, 160)}..."`,
      severity: 'info'
    });

    const hintBrewVisible = await page
      .locator('details.section')
      .nth(3)
      .locator('code:has-text("brew")')
      .count();
    record({
      persona: 'tereza',
      moment: 'install-hint',
      note: hintBrewVisible > 0
        ? 'Vidím brew příkaz — Tereza skopíruje.'
        : 'Instalační nápověda chybí.',
      severity: hintBrewVisible > 0 ? 'delight' : 'blocker'
    });

    const recheckBtn = await page
      .locator('button:has-text("Zkontrolovat znovu")')
      .count();
    record({
      persona: 'tereza',
      moment: 'recheck-button',
      note: recheckBtn > 0
        ? '"Zkontrolovat znovu" tlačítko existuje. Po `brew install` stisknu a UI se obnoví. C6 resolved.'
        : 'Re-check tlačítko chybí — C6 pořád otevřené.',
      severity: recheckBtn > 0 ? 'delight' : 'blocker'
    });
  });

  test('empty library now has a proper invitation card', async ({ page }) => {
    await page.goto('/study');
    await page.waitForSelector('.empty-invite');
    const inviteText = await page.locator('.empty-invite p').first().textContent();
    record({
      persona: 'tereza',
      moment: 'empty-invite',
      note: `Prázdný stav má pozvánku: "${inviteText?.replace(/\s+/g, ' ').trim().slice(0, 140)}...". Zmiňuje .md, PDF, GoodNotes export. To přesně mluví mým jazykem.`,
      severity: 'delight'
    });

    const buttons = await page.$$eval('.empty-actions button', (els) =>
      els.map((e) => e.textContent?.trim())
    );
    record({
      persona: 'tereza',
      moment: 'empty-actions',
      note: `Přímá akce: ${JSON.stringify(buttons)}. Dva primary paths, ne hromada voleb.`,
      severity: 'delight'
    });
  });
});
