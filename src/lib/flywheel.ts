// Flywheel feedback loop (Flywheel Standard §1 "feedback loop"). Dependency-free:
// raw fetch into the shared flywheel-core project. Two halves:
//   * analytics — the shared event taxonomy, CONSENT-GATED (datlino's About promises
//     "no telemetry without consent"); off until the user opts in.
//   * feedback — a user-initiated submission; always allowed (they chose to send it).
// Ships dark when unconfigured (no key) so it never breaks a local/offline build.
// Privacy: cookieless. visitor_id is an anonymous localStorage UUID; no note content
// ever leaves the device — only event names + coarse props.

const APP = 'datlino'
const URL = import.meta.env.VITE_FLYWHEEL_URL ?? 'https://chgwirzbpspoaoposuwg.supabase.co'
const KEY = import.meta.env.VITE_FLYWHEEL_KEY ?? '' // publishable key; empty → ships dark

const CONSENT_KEY = 'datlino:analytics-consent'
const VISITOR_KEY = 'fw:vid'

function ls(): Storage | null {
  try {
    return typeof localStorage !== 'undefined' ? localStorage : null
  } catch {
    return null
  }
}

export function analyticsEnabled(): boolean {
  return ls()?.getItem(CONSENT_KEY) === 'on'
}

export function setAnalyticsConsent(on: boolean): void {
  ls()?.setItem(CONSENT_KEY, on ? 'on' : 'off')
}

function visitorId(): string {
  const store = ls()
  if (!store) return 'anon'
  let v = store.getItem(VISITOR_KEY)
  if (!v) {
    v = crypto.randomUUID()
    store.setItem(VISITOR_KEY, v)
  }
  return v
}

async function insert(table: string, row: Record<string, unknown>): Promise<boolean> {
  if (!KEY) return false // unconfigured → no-op
  try {
    const res = await fetch(`${URL}/rest/v1/${table}`, {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        apikey: KEY,
        authorization: `Bearer ${KEY}`,
        prefer: 'return=minimal',
      },
      body: JSON.stringify({ app: APP, ...row }),
    })
    return res.ok
  } catch {
    return false // analytics/feedback must never throw into the app
  }
}

/** Shared taxonomy only — flywheel-core RLS rejects unknown event names. */
type TaxonomyEvent = 'page_view' | 'key_action' | 'conversion' | 'feedback_given' | 'error'

/** Passive analytics — silently no-ops unless the user opted in. */
export function track(event: TaxonomyEvent, props: Record<string, unknown> = {}): void {
  if (!analyticsEnabled()) return
  void insert('events', { event, props, visitor_id: visitorId(), created_at: new Date().toISOString() })
}

/** A finished typing session — the product's aha moment (a `conversion`). */
export function sessionCompleted(props: Record<string, unknown> = {}): void {
  track('conversion', props)
}

/** User-initiated feedback. Always sends (they chose to). Also logs feedback_given. */
export async function sendFeedback(input: {
  sean_ellis?: 'very' | 'somewhat' | 'not'
  score?: number
  text?: string
}): Promise<boolean> {
  const ok = await insert('feedback', {
    sean_ellis: input.sean_ellis ?? null,
    score: input.score ?? null,
    text: input.text ?? null,
    visitor_id: visitorId(),
    created_at: new Date().toISOString(),
  })
  void insert('events', {
    event: 'feedback_given',
    props: { sean_ellis: input.sean_ellis ?? null },
    visitor_id: visitorId(),
    created_at: new Date().toISOString(),
  })
  return ok
}

export const isConfigured = Boolean(KEY)
