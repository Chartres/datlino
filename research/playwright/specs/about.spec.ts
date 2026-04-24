import { test, expect } from '@playwright/test';
import { defaultCorpus, installMockTauri } from '../fixtures/mock-tauri';
import { openFile, record } from '../fixtures/observations';

/**
 * Pája checks what's new in this build. /about renders the
 * project-wide CHANGELOG.md baked into the Rust binary. Any persona
 * can use it; we route Pája's voice here because she's the one most
 * likely to read release notes.
 */
test.describe('/about — changelog surface', () => {
  test.beforeEach(async ({ page }) => {
    await installMockTauri(
      page,
      defaultCorpus({
        sessionsCompleted: 3,
        totalXp: 120,
        streak: 3
      })
    );
    openFile('paja', 'Co je nového v tomhle buildu?');
  });

  test('renders a non-empty changelog with headings and links', async ({ page }) => {
    await page.goto('/about');

    // get_version is not routed in the mock, so the header may show "—".
    // Still: the changelog markdown should render headings and lists.
    await page.waitForSelector('article.changelog');

    const h1Count = await page.locator('article.changelog h1').count();
    const h2Count = await page.locator('article.changelog h2').count();
    const listCount = await page.locator('article.changelog ul').count();

    record({
      persona: 'paja',
      moment: 'about-renders',
      note: `Změnové hlavičky: ${h1Count} h1 / ${h2Count} h2 / ${listCount} ul. Markdown se renderuje v aplikaci jako na webu.`,
      severity: h2Count > 0 && listCount > 0 ? 'delight' : 'confusion'
    });

    // The most recent section should be about the IA reorg.
    const bodyText = await page.locator('article.changelog').textContent();
    record({
      persona: 'paja',
      moment: 'about-top-section',
      note: bodyText?.includes('IA reorg')
        ? 'Nahoře vidím IA reorg položku — nejnovější změnu, kterou jsem právě zažila.'
        : 'Nejnovější změnu v changelogu jsem nenašla.',
      severity: bodyText?.includes('IA reorg') ? 'delight' : 'confusion'
    });
  });
});
