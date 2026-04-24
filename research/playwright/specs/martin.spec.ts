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

  test('settings page reveals provider + OCR + rephrase', async ({ page }) => {
    await page.goto('/settings');
    await page.waitForSelector('h2');

    const tiles = await page.$$eval('.tile strong', (els) =>
      els.map((e) => e.textContent?.trim())
    );
    record({
      persona: 'martin',
      moment: 'provider-tiles',
      note: `Dostupné providery: ${JSON.stringify(tiles)}. Oceňuji, že "Lokální Candle" je první třídní volba a ne greyed-out.`,
      severity: 'delight'
    });

    // Click Candle.
    await page.click('.tile:has-text("Lokální Candle")');
    await page.waitForTimeout(500);
    record({
      persona: 'martin',
      moment: 'switch-to-candle',
      note: `Kliknutí na Candle mě nic nestálo — v tomhle prostředí model není stažený, ale UI to hlásí jako úspěch. V realitě by to mělo ukázat progress stahování.`,
      severity: 'confusion'
    });

    // Check the OCR section: the mock says both binaries are missing.
    const ocrBadges = await page.$$eval('.binlist li', (els) =>
      els.map((e) => e.textContent?.replace(/\s+/g, ' ').trim())
    );
    record({
      persona: 'martin',
      moment: 'ocr-section',
      note: `OCR stav: ${JSON.stringify(ocrBadges)}. Instalační nápovědu vidím: brew / apt. Dobrý.`,
      severity: 'delight'
    });

    // Rephrase card — Anthropic key + console deep-link.
    const anthropicLabel = await page
      .locator('button.secondary:has-text("Anthropic")')
      .count();
    record({
      persona: 'martin',
      moment: 'anthropic-deep-link',
      note: anthropicLabel > 0
        ? 'Tlačítko "Otevřít Anthropic Console" existuje. OAuth flow stále chybí, ale console deep-link je OK cesta pro teď.'
        : 'Nenašel jsem deep-link na Anthropic Console — škoda.',
      severity: anthropicLabel > 0 ? 'delight' : 'confusion'
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
