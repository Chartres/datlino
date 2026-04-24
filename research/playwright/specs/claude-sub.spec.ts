import { test, expect } from '@playwright/test';
import { defaultCorpus, installMockTauri } from '../fixtures/mock-tauri';
import { openFile, record } from '../fixtures/observations';

/**
 * Claude subscription auth (AI-001) — the student's Claude Pro/Max plan
 * should unlock Remix mode without a second BYOK key. The Rust side
 * detects Claude Code credentials and prefers Bearer OAuth over the
 * keychain API key. UI surfaces the detected state.
 *
 * Three scenarios:
 *   1. No Claude Code installed → BYOK fallback message is clear.
 *   2. Claude Code live subscription detected → "Aktivní" pill + message.
 *   3. Expired subscription → "Vypršelo" pill + guidance to re-login.
 */

test.describe('Claude subscription auth', () => {
  test('no Claude Code installed → BYOK path surfaces', async ({ page }) => {
    await installMockTauri(page, defaultCorpus());
    openFile(
      'jonas',
      'Nemám Claude Code nainstalovaný. Co vidím v Remix sekci?'
    );
    await page.goto('/settings');
    await page.waitForSelector('details.section');
    // Expand Remix (section 3, index 2).
    await page.locator('details.section').nth(2).locator('summary').click();
    await page.waitForTimeout(100);

    const body = await page.locator('details.section').nth(2).locator('.section-body').textContent();
    record({
      persona: 'jonas',
      moment: 'remix-no-sub',
      note: `Remix sekce mluví: "${body?.replace(/\s+/g, ' ').trim().slice(0, 200)}...". Zmiňuje claude login a BYOK fallback explicitně.`,
      severity:
        body?.includes('claude login') && body?.includes('BYOK') ? 'delight' : 'confusion'
    });
  });

  test('live Pro subscription detected → "Aktivní" pill shown', async ({ page }) => {
    await installMockTauri(page, defaultCorpus());
    // Flip the mock's Claude-sub state to simulate a live Pro user.
    await page.addInitScript(() => {
      const s = (window as any).__MOCK_STATE__;
      if (s) {
        s.claudeSubscription = {
          detected: true,
          expired: false,
          subscription_type: 'pro',
          source: 'file'
        };
      }
    });
    openFile(
      'paja',
      'Mám Claude Pro. Doufám, že si Datlino načte můj login a nebudu muset platit dvakrát.'
    );

    await page.goto('/settings');
    await page.waitForSelector('details.section');
    await page.locator('details.section').nth(2).locator('summary').click();
    await page.waitForTimeout(100);

    const pills = await page.locator('.auth-card .pill').allTextContents();
    record({
      persona: 'paja',
      moment: 'remix-live-sub',
      note: `Auth-card ukazuje pills: ${JSON.stringify(pills)}. Měla by být "Aktivní" pro live subscription.`,
      severity: pills.some((p) => p.includes('Aktivní')) ? 'delight' : 'confusion'
    });

    // And the summary line shows the subscription type.
    const secSub = await page.locator('details.section').nth(2).locator('.sec-sub').textContent();
    record({
      persona: 'paja',
      moment: 'remix-sec-sub',
      note: `Nadpis sekce říká: "${secSub?.trim()}". Rychlý indikátor stavu přímo v collapsed summary.`,
      severity: secSub?.includes('pro') || secSub?.includes('Pro') ? 'delight' : 'confusion'
    });
  });

  test('expired subscription → "Vypršelo" pill + re-login nudge', async ({ page }) => {
    await installMockTauri(page, defaultCorpus());
    await page.addInitScript(() => {
      const s = (window as any).__MOCK_STATE__;
      if (s) {
        s.claudeSubscription = {
          detected: true,
          expired: true,
          subscription_type: 'max',
          source: 'keychain'
        };
      }
    });
    openFile(
      'martin',
      'Odhlásil jsem se před týdnem. Jak Datlino řeší expirovaný token?'
    );

    await page.goto('/settings');
    await page.waitForSelector('details.section');
    await page.locator('details.section').nth(2).locator('summary').click();
    await page.waitForTimeout(100);

    const pills = await page.locator('.auth-card .pill').allTextContents();
    const body = await page.locator('.auth-card').first().textContent();
    record({
      persona: 'martin',
      moment: 'remix-expired',
      note: `Pills: ${JSON.stringify(pills)}. Body zmiňuje claude login: ${body?.includes('claude login')}.`,
      severity:
        pills.some((p) => p.includes('Vypršelo')) && body?.includes('claude login')
          ? 'delight'
          : 'blocker'
    });
  });
});
