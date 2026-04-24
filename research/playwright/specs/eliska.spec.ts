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
      note: `Top nav: ${JSON.stringify(navLinks)}. She sees "Trénink" and clicks it.`,
      severity: 'info'
    });

    await page.click('nav a[href="/practice"]');
    await page.waitForURL('**/practice');

    // She sees six tiles. Which one is for "I don't know how to type"?
    const modeTitles = await page.$$eval('.mode h3', (els) =>
      els.map((e) => e.textContent?.trim())
    );
    record({
      persona: 'eliska',
      moment: 'practice-picker',
      note: `She sees these six tiles: ${JSON.stringify(modeTitles)}. "Úvodní lekce" is the first one — good. But also sees "Moje materiály", "Zahřívání", "Diakritika", "Tvá slabá místa", "Mix" — six choices is a lot for a new user.`,
      severity: 'confusion'
    });

    // She clicks Úvodní lekce. What happens?
    const introTile = page.locator('.mode').filter({ hasText: 'Úvodní lekce' });
    await introTile.click();
    record({
      persona: 'eliska',
      moment: 'pick-intro',
      note: 'Clicking the tile *selects* it but doesn\'t open anything. She has to also click "Začít". Two-click pattern for something that should be one-click for her persona.',
      severity: 'confusion'
    });

    await page.click('button.primary:has-text("Začít")');
    await page.waitForURL('**/practice/intro');

    // Lesson list. Is it clear where to start?
    const lessonTitles = await page.$$eval('.lessons h3', (els) =>
      els.map((e) => e.textContent?.trim())
    );
    record({
      persona: 'eliska',
      moment: 'lesson-list',
      note: `Sees ${lessonTitles.length} lessons. First one = "${lessonTitles[0]}". Targets "Cíl: 10 WPM · 92% přesnost" mean nothing to her on day 1 — she doesn't know what WPM means yet.`,
      severity: 'confusion'
    });

    // She clicks "Začít" on the first lesson.
    const firstStart = page.locator('.lessons li button.primary').first();
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

  test('settings page feels out of bounds for her', async ({ page }) => {
    await page.goto('/settings');
    await page.waitForSelector('h2');
    const sections = await page.$$eval('section h3', (els) =>
      els.map((e) => e.textContent?.trim())
    );
    record({
      persona: 'eliska',
      moment: 'settings-scan',
      note: `Settings shows: ${JSON.stringify(sections)}. She sees "Aktuální provider", "Vyber provider", "Cohere API klíč", "OCR", "Rephrase mode", "Anthropic klíč". She'd bounce. None of these speak to a 14-year-old beginner.`,
      severity: 'blocker'
    });
  });
});
