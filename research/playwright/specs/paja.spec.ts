import { test, expect } from '@playwright/test';
import { defaultCorpus, installMockTauri } from '../fixtures/mock-tauri';
import { openFile, record } from '../fixtures/observations';

/**
 * Pája, 17, maturita in May. Knows how to type well enough. Wants the
 * app to earn its keep on actual dějepis material. Will judge this on:
 *   - Can she point it at her notes folder and get sensible sessions?
 *   - Does Exam-Prep actually feel like exam prep?
 */
test.describe('Pája, 17 — maturita student', () => {
  test.beforeEach(async ({ page }) => {
    await installMockTauri(
      page,
      defaultCorpus({
        documents: [
          {
            id: 1,
            source_path: '/Users/paja/skola/dejepis/habsburkove.md',
            kind: 'md',
            text: 'Habsburkové vládli v Čechách od roku 1526. Ferdinand I. nastoupil na trůn. Marie Terezie prosadila rozsáhlé reformy. Bitva na Bílé hoře proběhla roku 1620. Následovala rekatolizace zemí. Jozef II. zrušil nevolnictví v roce 1781.'
          },
          {
            id: 2,
            source_path: '/Users/paja/skola/dejepis/chemie-periodicka-soustava.md',
            kind: 'md',
            text: 'Periodickou soustavu sestavil Dmitrij Mendělejev v roce 1869. Uspořádal prvky podle rostoucí atomové hmotnosti. Dnešní soustava je řazena podle protonového čísla.'
          }
        ],
        watchedRoots: ['/Users/paja/skola/dejepis']
      })
    );
    openFile(
      'paja',
      'Přidala jsem si dějepis složku. Teď chci uvidět, jestli mi to pomůže se učit na maturitu.'
    );
  });

  test('lands on a library with her docs visible', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('section');

    const docs = await page.$$eval('.docs li .doc-name', (els) =>
      els.map((e) => e.textContent?.trim())
    );
    record({
      persona: 'paja',
      moment: 'library-docs',
      note: `Library shows documents: ${JSON.stringify(docs)}. Dvě soubory — to je málo na reálnou maturitu. V realitě bych měla 30+ PDF.`,
      severity: 'info'
    });

    // She searches "chemie" — which lives only in the filename.
    await page.fill('input[placeholder*="Habsburkové"]', 'chemie');
    await page.click('button.primary:has-text("Hledat")');
    await page.waitForTimeout(500);

    const hitCount = await page.locator('.hits li').count();
    record({
      persona: 'paja',
      moment: 'chemie-search',
      note: `"chemie" found ${hitCount} hits. Předchozí verze vracela 1; pokud backend dopadá na název souboru, měla bych vidět víc. V Playwright mocku hledám jen v textu → 0. V reálné aplikaci by to mělo být lepší.`,
      severity: hitCount === 0 ? 'confusion' : 'delight'
    });

    await page.screenshot({
      path: 'research/playwright/screenshots/paja-library.png',
      fullPage: true
    });
  });

  test('drills a whole document directly', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.docs li');
    const habsDoc = page.locator('.docs li').filter({ hasText: 'habsburkove' });
    await habsDoc.locator('button').click();
    await page.waitForURL('**/practice/session');
    await page.waitForSelector('.typing-surface');

    const firstChar = await page.locator('.char.cursor').textContent();
    record({
      persona: 'paja',
      moment: 'drill-doc-first-char',
      note: `Klikla jsem na dokument a rovnou jsem v tréninku. První znak: "${firstChar}". To je ta věta, kterou budu psát. Jednoklik-to-flow je dobrý.`,
      severity: 'delight'
    });
  });

  test('tries exam-prep with a natural-language topic', async ({ page }) => {
    await page.goto('/practice');

    await page.click('.mode:has-text("Moje materiály")');
    await page.click('.strategy:has-text("Příprava na zkoušku")');
    await page.fill(
      'input[type="text"]',
      'Habsburkové a jejich reformy v 17. a 18. století'
    );
    await page.click('button.primary:has-text("Začít")');
    await page.waitForTimeout(800);

    const urlAfter = page.url();
    record({
      persona: 'paja',
      moment: 'exam-prep-result',
      note: `Po kliknutí jsem skončila na ${urlAfter}. Pokud cesta obsahuje /session, funguje to; jinak buď nemám dost materiálu nebo jsem uvízla.`,
      severity: urlAfter.includes('/session') ? 'delight' : 'confusion'
    });
  });
});
