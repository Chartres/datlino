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

  test('library shows no documents when library is empty', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('section');
    const docCount = await page.locator('.docs li').count();
    record({
      persona: 'tereza',
      moment: 'empty-library',
      note: `Dokumentů: ${docCount}. Prázdný stav je hezky schovaný — ale chybí mi nápověda "vlož PDF z GoodNotes". Přímá pozvánka k akci.`,
      severity: 'confusion'
    });
  });
});
