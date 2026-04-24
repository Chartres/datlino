import { test, expect } from '@playwright/test';
import { defaultCorpus, installMockTauri } from '../fixtures/mock-tauri';
import { openFile, record } from '../fixtures/observations';

/**
 * Lucie, 19, heavy corpus. Simulates 15 documents — enough to stress the
 * document picker layout but not so many that the mock can't hold them.
 */
test.describe('Lucie, 19 — heavy corpus', () => {
  test.beforeEach(async ({ page }) => {
    const documents = Array.from({ length: 15 }, (_, i) => ({
      id: i + 1,
      source_path: `/Users/lucie/thesis/source_${String(i + 1).padStart(2, '0')}.md`,
      kind: 'md',
      text: `Zdrojový text číslo ${i + 1}. Habsburkové vládli v Čechách mnoho let. \
Marie Terezie reformovala školství. Bitva u Moháče roku 1526 předznamenala \
nástup Habsburků. Další věta pro kontext historie.`
    }));
    await installMockTauri(page, defaultCorpus({ documents }));
    openFile('lucie', 'Patnáct dokumentů, semestrálka. Prosím ať je to použitelné.');
  });

  test('library renders 15 document tiles without collapsing', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.docs li');
    const count = await page.locator('.docs li').count();
    expect(count).toBe(15);
    record({
      persona: 'lucie',
      moment: 'doc-grid',
      note: `Grid ukazuje ${count} dokumentů. Mřížka se naskládá v ~3 sloupcích podle šířky. To funguje pro 15 souborů.`,
      severity: 'delight'
    });

    // But a real thesis library is 200+ — is there filtering or grouping?
    record({
      persona: 'lucie',
      moment: 'filter-missing',
      note: 'Chybí filtr / vyhledávání v seznamu dokumentů. Při 200 souborech by se mi dlouho scrollovalo.',
      severity: 'confusion'
    });

    await page.screenshot({
      path: 'research/playwright/screenshots/lucie-library.png',
      fullPage: true
    });
  });

  test('exam-prep across many docs returns grouped content', async ({ page }) => {
    await page.goto('/practice');
    await page.click('.mode:has-text("Moje materiály")');
    await page.click('.strategy:has-text("Příprava na zkoušku")');
    await page.fill(
      'input[type="text"]',
      'Habsburkové Marie Terezie reformy'
    );
    await page.click('button.primary:has-text("Začít")');
    await page.waitForTimeout(600);

    const onSession = page.url().includes('/session');
    record({
      persona: 'lucie',
      moment: 'exam-prep-route',
      note: onSession
        ? 'ExamPrep mě pustila do session s relevantním obsahem. Dobré pro práci na semestrálce.'
        : 'ExamPrep se neposunula do session — něco s BM25 a málo materiálem.',
      severity: onSession ? 'delight' : 'confusion'
    });
  });
});
