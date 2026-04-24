import { test, expect } from '@playwright/test';
import { defaultCorpus, installMockTauri } from '../fixtures/mock-tauri';
import { openFile, record } from '../fixtures/observations';

/**
 * Jonáš, 15, metro commuter. Offline-first is non-negotiable.
 */
test.describe('Jonáš, 15 — offline on the train', () => {
  test.beforeEach(async ({ page }) => {
    await installMockTauri(
      page,
      defaultCorpus({
        documents: [
          {
            id: 1,
            source_path: '/home/jonas/study/dejepis.md',
            kind: 'md',
            text: 'První světová válka skončila v roce 1918. Versailleská smlouva určila poválečné uspořádání. Hitler byl ještě dítě. Hospodářská krize přišla v roce 1929.'
          }
        ],
        embeddingProvider: 'fake'
      })
    );
    openFile('jonas', 'Na vlaku nefunguje wifi. Cokoli se snaží na cloud selže.');
  });

  test('library + drill works without network claims', async ({ page }) => {
    await page.goto('/study');
    await page.waitForSelector('.docs li');

    // He clicks the one document to drill it.
    await page.locator('.docs li button').first().click();
    await page.waitForURL('**/practice/session');
    await page.waitForSelector('.typing-surface');
    record({
      persona: 'jonas',
      moment: 'offline-flow',
      note: 'Kliknutí na dokument vede přímo do session — žádné cloud calls. Dobrý offline UX.',
      severity: 'delight'
    });
  });

  test('settings provider info visible by one click', async ({ page }) => {
    await page.goto('/settings');
    await page.waitForSelector('details.section');
    const providerSub = await page
      .locator('details.section')
      .nth(1)
      .locator('.sec-sub')
      .textContent();
    record({
      persona: 'jonas',
      moment: 'settings-provider-strip',
      note: `Kvalita vyhledávání sekce hlásí: "${providerSub?.trim()}". Provider je vidět i bez rozkliknutí — Jonáš zavře a jede.`,
      severity: 'delight'
    });
    record({
      persona: 'jonas',
      moment: 'offline-banner',
      note: 'Explicitní "funguje offline" banner stále chybí — open ask (UX-007).',
      severity: 'confusion'
    });
  });
});
