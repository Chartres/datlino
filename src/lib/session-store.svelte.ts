import type { SessionPlan, SessionSummary } from './types';

// Session passes between the selector page, the typing engine, and the
// summary page via this in-memory store. Avoids stuffing large keystroke
// logs into URL query params.
export const currentSession = $state<{
  plan: SessionPlan | null;
  summary: SessionSummary | null;
}>({
  plan: null,
  summary: null
});
