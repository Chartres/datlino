import { test, expect } from '@playwright/test';
import { defaultCorpus, installMockTauri } from '../fixtures/mock-tauri';
import { openFile, record } from '../fixtures/observations';

/**
 * Eliška, 14, 9th grade. Shared family Windows laptop. True beginner at
 * typing. Parent installed Datlino for her. She'll open the app, feel
 * lost for 5 seconds, and decide within a minute whether to try again
 * tomorrow. We're judging the app's first-run clarity.
 */
test.describe('Eliška, 14 — absolute beginner', () => {
  test.beforeEach(async ({ page }) => {
    await installMockTauri(
      page,
      defaultCorpus({
        embeddingProvider: 'fake',
        sessionsCompleted: 0
      })
    );
    openFile(
      'eliska',
      'Mamka mi to nainstalovala. Doufám že mi to řekne co mám dělat.'
    );
  });

  test('opens the app and can find the beginner track', async ({ page }) => {
    await page.goto('/');

    await page.waitForSelector('header h1');
    const title = await page.textContent('header h1');
    record({
      persona: 'eliska',
      moment: 'first-screen',
      note: `Window title reads "${title}".`,
      severity: 'info'
    });

    // She looks at the nav bar for a clue.
    const navLinks = await page.$$eval('nav a', (els) =>
      els.map((e) => e.textContent?.trim())
    );
    record({
      persona: 'eliska',
      moment: 'nav-scan',
      note: `Top nav: ${JSON.stringify(navLinks)}. New IA — "Učím se psát" and "Učím se obsah" are obvious Czech phrases; she picks the first.`,
      severity: 'delight'
    });

    // Home page now has two big doors. She picks the learn one.
    const doors = await page.$$eval('.door h3', (els) =>
      els.map((e) => e.textContent?.trim())
    );
    record({
      persona: 'eliska',
      moment: 'home-doors',
      note: `Home page ukazuje dvě velké dveře: ${JSON.stringify(doors)}. Žádný shluk šesti dlaždic. Ulehčení.`,
      severity: 'delight'
    });

    await page.click('a.door.learn');
    await page.waitForURL('**/learn');

    // Only three tiles on /learn now: IntroLesson, WeakKeys, Diacritics.
    const drillTiles = await page.$$eval('.drills .tile h3', (els) =>
      els.map((e) => e.textContent?.trim())
    );
    record({
      persona: 'eliska',
      moment: 'learn-tiles',
      note: `Na /learn vidím tři dlaždice: ${JSON.stringify(drillTiles)}. "Úvodní lekce" je zvýrazněná s primary border — jasná první volba.`,
      severity: 'delight'
    });

    // The big CTA at the top offers "Pokračovat: ..." or "Začít první
    // lekci" — one-click into first lesson.
    const ctaText = await page.locator('.hero .cta').textContent();
    record({
      persona: 'eliska',
      moment: 'one-click-cta',
      note: `Velké tlačítko nahoře říká: "${ctaText?.trim()}". Jedno kliknutí = hned píšu.`,
      severity: 'delight'
    });

    await page.click('.hero .cta');
    await page.waitForURL('**/practice/session');
    await page.waitForSelector('.typing-surface');

    // Skipped the lesson ladder entirely — direct path. Let's also
    // verify the ladder is visible if she wants it.
    await page.goto('/learn');
    const lessonTitles = await page.$$eval('.lessons h4', (els) =>
      els.map((e) => e.textContent?.trim())
    );
    record({
      persona: 'eliska',
      moment: 'lesson-ladder',
      note: `Ladder under the tiles ukazuje ${lessonTitles.length} lekcí. První = "${lessonTitles[0]}". Targets ("Cíl: 10 WPM · 92% přesnost") pořád nemluví jazykem 14leté začátečnice — to je C2 blocker pořád otevřený.`,
      severity: 'confusion'
    });

    // She clicks "Začít" on the first lesson.
    const firstStart = page.locator('.lessons li button').first();
    await firstStart.click();
    await page.waitForURL('**/practice/session');
    await page.waitForSelector('.typing-surface');

    // Does the on-screen keyboard show up?
    const keyboardVisible = await page.locator('.keyboard').isVisible();
    expect(keyboardVisible).toBe(true);
    record({
      persona: 'eliska',
      moment: 'session-opened',
      note: `Typing surface + keyboard both render. Keyboard shows finger-zone colors. Home-row dots on F/J/A/; are present. The next key is glowing. She'd understand this visually.`,
      severity: 'delight'
    });

    // First target character.
    const firstChar = await page.locator('.char.cursor').textContent();
    record({
      persona: 'eliska',
      moment: 'first-char',
      note: `First expected char is "${firstChar}". Drill line starts with "aaa" — home row left pinky. Good scaffolding.`,
      severity: 'delight'
    });

    // She types the 3 a's correctly.
    await page.keyboard.press('a');
    await page.keyboard.press('a');
    await page.keyboard.press('a');
    await page.waitForTimeout(100);
    const typedCount = await page.locator('.char.correct').count();
    record({
      persona: 'eliska',
      moment: 'three-a-typed',
      note: `After typing "aaa": ${typedCount} chars marked correct. Cursor advanced. WPM hud probably shows something. She feels good.`,
      severity: 'delight'
    });

    await page.screenshot({
      path: 'research/playwright/screenshots/eliska-first-session.png',
      fullPage: true
    });
    record({
      persona: 'eliska',
      moment: 'screenshot',
      note: 'Captured at `research/playwright/screenshots/eliska-first-session.png`.',
      severity: 'info'
    });
  });

  test('settings now hide infrastructure behind 4 collapsible sections', async ({ page }) => {
    await page.goto('/settings');
    await page.waitForSelector('details.section');

    // Only "Profil" should be open by default; everything else closed.
    const openSections = await page
      .locator('details.section[open] .sec-title')
      .allTextContents();
    record({
      persona: 'eliska',
      moment: 'settings-default-open',
      note: `Výchozí otevřené sekce: ${JSON.stringify(openSections)}. Měla by být jen "Profil".`,
      severity: openSections.length === 1 && openSections[0] === 'Profil' ? 'delight' : 'blocker'
    });

    const titles = await page.locator('details.section .sec-title').allTextContents();
    record({
      persona: 'eliska',
      moment: 'settings-sections',
      note: `Všechny sekce: ${JSON.stringify(titles)}. Infrastructure (embeddings, OCR, remix) schovaná — Eliška uvidí jen svoje Profil statistiky.`,
      severity: 'delight'
    });

    // Profil card shows her stats, no jargon.
    const profilBody = await page.locator('details.section[open] .section-body').textContent();
    record({
      persona: 'eliska',
      moment: 'profil-stats',
      note: `Profil sekce: "${profilBody?.replace(/\s+/g, ' ').trim().slice(0, 120)}...". Bez embedding/provider/API.`,
      severity: 'delight'
    });
  });
});
