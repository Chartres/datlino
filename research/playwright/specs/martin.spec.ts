import { test, expect } from '@playwright/test';
import { defaultCorpus, installMockTauri } from '../fixtures/mock-tauri';
import { openFile, record } from '../fixtures/observations';

/**
 * Martin, 18, Linux power user. Cares about provider choice, local-first,
 * and whether the app respects his technical ability. Opens Settings
 * first, like a nerd.
 */
test.describe('Martin, 18 — technical power user', () => {
  test.beforeEach(async ({ page }) => {
    await installMockTauri(
      page,
      defaultCorpus({
        embeddingProvider: 'fake',
        documents: [
          {
            id: 1,
            source_path: '/home/martin/study/grammar.md',
            kind: 'md',
            text: 'Present perfect connects past to present. I have lived here for five years. She has finished. They have arrived. We just sat down.'
          }
        ]
      })
    );
    openFile('martin', 'Nejdřív Settings. Chci vědět co to provider dělá.');
  });

  test('settings are 4 collapsible sections — no overload by default', async ({ page }) => {
    await page.goto('/settings');
    await page.waitForSelector('details.section');

    const titles = await page.locator('details.section .sec-title').allTextContents();
    record({
      persona: 'martin',
      moment: 'settings-sections',
      note: `Sekce: ${JSON.stringify(titles)}. Čtyři. Hierarchie dává smysl.`,
      severity: 'delight'
    });

    // Open Kvalita vyhledávání, verify provider tiles.
    await page.locator('details.section').nth(1).locator('summary').click();
    await page.waitForTimeout(150);
    const tiles = await page.locator('.tiles .tile strong').allTextContents();
    record({
      persona: 'martin',
      moment: 'provider-tiles',
      note: `Dostupné providery: ${JSON.stringify(tiles)}. "Lokální Candle" je první — dobré defaulty.`,
      severity: 'delight'
    });

    // Open Remix → verify the Claude subscription card exists
    await page.locator('details.section').nth(2).locator('summary').click();
    await page.waitForTimeout(150);
    const subCardHeading = await page.locator('.auth-card h4').first().textContent();
    record({
      persona: 'martin',
      moment: 'claude-sub-card',
      note: `Remix sekce má auth card: "${subCardHeading?.trim()}". Claude subscription je primární auth, BYOK fallback. Konečně.`,
      severity: 'delight'
    });

    // Open OCR, check for "Zkontrolovat znovu" (Tereza's C6 fix).
    await page.locator('details.section').nth(3).locator('summary').click();
    await page.waitForTimeout(150);
    const recheck = await page.locator('button:has-text("Zkontrolovat znovu")').count();
    record({
      persona: 'martin',
      moment: 'ocr-recheck',
      note: recheck > 0
        ? '"Zkontrolovat znovu" tlačítko přítomno — C6 fixed.'
        : 'Chybí re-check tlačítko.',
      severity: recheck > 0 ? 'delight' : 'blocker'
    });

    await page.screenshot({
      path: 'research/playwright/screenshots/martin-settings.png',
      fullPage: true
    });
  });

  test('alpha slider is now buried under "Pokročilé" on /study', async ({ page }) => {
    await page.goto('/study');
    await page.waitForSelector('.strategies');

    // Has to expand "Pokročilé" — that's the whole point: hide it from
    // Eliška, surface it for Martin.
    const detailsBefore = await page.locator('.advanced').evaluate((el) => (el as HTMLDetailsElement).open);
    record({
      persona: 'martin',
      moment: 'advanced-collapsed-by-default',
      note: `<details class="advanced"> open=${detailsBefore} při načtení. Skrytí α-slideru za "Pokročilé" je správně — Eliška to neuvidí, já ano.`,
      severity: 'delight'
    });

    await page.click('.advanced summary');
    await page.locator('input[type="checkbox"]').first().check();
    await page.waitForTimeout(150);
    const sliderVisible = await page.locator('input[type="range"]').isVisible();
    record({
      persona: 'martin',
      moment: 'alpha-slider',
      note: sliderVisible
        ? 'Po expand "Pokročilé" + checkbox α-mix se zobrazil slider s popiskem "obsah · trénink". Srozumitelné, dvě kliknutí navíc je pro power-usera v pohodě.'
        : 'α-slider se neobjevil — bug.',
      severity: sliderVisible ? 'delight' : 'blocker'
    });
  });
});
