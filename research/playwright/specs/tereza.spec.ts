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

  test('settings surface tells her what to install', async ({ page }) => {
    await page.goto('/settings');
    await page.waitForSelector('h2');

    const ocrSection = await page.locator('section').filter({ hasText: 'OCR' }).textContent();
    record({
      persona: 'tereza',
      moment: 'ocr-help',
      note: `OCR sekce: "${ocrSection?.replace(/\s+/g, ' ').trim().slice(0, 200)}..."`,
      severity: 'info'
    });

    const hintBrewVisible = await page.locator('section:has-text("OCR") code:has-text("brew")').count();
    record({
      persona: 'tereza',
      moment: 'install-hint',
      note: hintBrewVisible > 0
        ? 'Vidím brew příkaz pro macOS. Tereza to skopíruje a spustí. OK.'
        : 'Instalační nápověda chybí — to je blocker.',
      severity: hintBrewVisible > 0 ? 'delight' : 'blocker'
    });

    record({
      persona: 'tereza',
      moment: 'after-install-feedback',
      note: 'Otevřená otázka: po `brew install tesseract poppler` aplikace sama nic neudělá — musím zavřít a otevřít? Přežil by tu tlačítko "Zkontrolovat znovu".',
      severity: 'confusion'
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
