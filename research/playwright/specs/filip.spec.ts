import { test, expect } from '@playwright/test';
import { defaultCorpus, installMockTauri } from '../fixtures/mock-tauri';
import { openFile, record } from '../fixtures/observations';

/**
 * Filip, 13, absolute typing beginner. Colors + visible guidance matter
 * enormously. The on-screen keyboard is the whole point for him.
 */
test.describe('Filip, 13 — keyboard-first beginner', () => {
  test.beforeEach(async ({ page }) => {
    await installMockTauri(page, defaultCorpus());
    openFile('filip', 'Ukaž mi kde jsou prsty. Já na klávesnici nevidím.');
  });

  test('the on-screen keyboard teaches him where his fingers go', async ({ page }) => {
    await page.goto('/learn');
    await page.waitForSelector('.hero .cta');
    await page.click('.hero .cta');
    await page.waitForURL('**/practice/session');
    await page.waitForSelector('.keyboard');

    // Count finger-zone colors visible in the legend.
    const legendLabels = await page.$$eval('.legend-item span:last-child', (els) =>
      els.map((e) => e.textContent?.trim())
    );
    record({
      persona: 'filip',
      moment: 'keyboard-legend',
      note: `Legenda prstů: ${JSON.stringify(legendLabels)}. Osm prstů + palec. Barvy spojují prst s klávesou — to mi dává smysl.`,
      severity: 'delight'
    });

    // The active key (the one he's supposed to press next) should have
    // the .active class and a visible glow.
    const activeKey = page.locator('.key.active').first();
    await expect(activeKey).toBeVisible();
    const label = await activeKey.textContent();
    record({
      persona: 'filip',
      moment: 'highlighted-key',
      note: `Svítí mi klávesa "${label?.trim()}". Hned vím kam dát prst.`,
      severity: 'delight'
    });

    // Home-row dots on F/J (the bumps).
    const dots = await page.locator('.home-dot').count();
    record({
      persona: 'filip',
      moment: 'home-row-dots',
      note: `Vidím ${dots} puntíků na klávesách — stejně jako na fyzické klávesnici F a J.`,
      severity: 'delight'
    });

    await page.screenshot({
      path: 'research/playwright/screenshots/filip-keyboard-hint.png',
      fullPage: true
    });

    // He tries pressing the key.
    await page.keyboard.press('a');
    await page.waitForTimeout(100);
    const firstCorrect = await page.locator('.char.correct').first().textContent();
    record({
      persona: 'filip',
      moment: 'first-keystroke',
      note: `Napsal jsem "a" a znak zčernal. To vypadá dobře. (Byl "${firstCorrect}".)`,
      severity: 'delight'
    });

    // But: he's looking for a ROADMAP — which key comes AFTER the next?
    record({
      persona: 'filip',
      moment: 'missing-roadmap',
      note: `Otázka od Filipa: "A co bude další klávesa potom?" Aplikace svítí jen na *příští* klávesu. Nabídnout dopředný náhled by byla pro něj pomoc.`,
      severity: 'confusion'
    });

    // Try a character that needs a shift key (uppercase A).
    record({
      persona: 'filip',
      moment: 'shift-handling-hypothetical',
      note: `Pokud by přišlo velké "A", očekával by zvýraznění klávesy Shift vlevo v barvě L-malíčku. Ten code path existuje (.shift-glow) — ale test by chtěl lekci která ho spouští.`,
      severity: 'info'
    });
  });

  test('toggle hides the keyboard for independence training', async ({ page }) => {
    await page.goto('/learn');
    await page.click('.hero .cta');
    await page.waitForURL('**/practice/session');
    // New: calibration modal appears before the surface is interactive.
    const cal = page.locator('.calibration-modal');
    if (await cal.count()) await cal.locator('button.primary').click();
    await page.waitForSelector('.keyboard');

    await page.click('button:has-text("Skrýt klávesnici")');
    await expect(page.locator('.keyboard')).toHaveCount(0);
    record({
      persona: 'filip',
      moment: 'hide-keyboard',
      note: `Mohu klávesnici vypnout. Až si budu jistější, zkusím to bez ní. Dobrý přechod.`,
      severity: 'delight'
    });
  });
});
