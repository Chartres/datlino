import type { PracticeMode } from './types';

// Montessori mapping — each mode foregrounds one principle. The `principle`
// field is shown under the mode name so students (and parents) understand
// *why* a mode exists, not just what it does.
export type ModeMeta = {
  code: PracticeMode;
  title: string;
  subtitle: string;
  principle: string;
  supportsAlpha: boolean;
  supportsQuery: boolean;
};

export const modes: ModeMeta[] = [
  {
    code: 'intro_lesson',
    title: 'Úvodní lekce',
    subtitle: 'Pro začátečníky: domovská řada → celá klávesnice → diakritika.',
    principle: 'Progressive disclosure — jedna dovednost za druhou.',
    supportsAlpha: false,
    supportsQuery: false
  },
  {
    code: 'content',
    title: 'Moje materiály',
    subtitle: 'Piš věty přímo ze svých poznámek.',
    principle: 'Prepared environment — drill na reálném studijním obsahu.',
    supportsAlpha: false,
    supportsQuery: true
  },
  {
    code: 'warmup',
    title: 'Zahřívání',
    subtitle: 'Krátké a jednoduché věty na rozjezd.',
    principle: 'Confidence-first — jednodušší před obtížnějším.',
    supportsAlpha: false,
    supportsQuery: false
  },
  {
    code: 'diacritics',
    title: 'Diakritika',
    subtitle: 'Izolovaný trénink č, š, ř, ě a spol.',
    principle: 'Isolation of difficulty — jedna dovednost v jeden čas.',
    supportsAlpha: false,
    supportsQuery: false
  },
  {
    code: 'weak_keys',
    title: 'Tvá slabá místa',
    subtitle: 'Věty hustě obsahující tvé nejslabší kombinace.',
    principle: 'Control of error — cílená oprava vlastních chyb.',
    supportsAlpha: false,
    supportsQuery: false
  },
  {
    code: 'hybrid',
    title: 'Mix',
    subtitle: 'Obsah podle tvého dotazu vyvážený s tréninkem slabin.',
    principle: 'Student-led — posuvníkem si sám určíš poměr.',
    supportsAlpha: true,
    supportsQuery: true
  },
  {
    code: 'dictation',
    title: 'Diktát',
    subtitle: 'Datlino čte věty nahlas, ty je píšeš. Pauzuje, když nestíháš.',
    principle: 'Multimodality — sluch + svaly + vizuální stopa zároveň.',
    supportsAlpha: false,
    supportsQuery: true
  }
];

export function modeByCode(code: string): ModeMeta | undefined {
  return modes.find((m) => m.code === code);
}

export const badgeLabels: Record<string, string> = {
  first_session: 'První sezení',
  five_sessions: '5 sezení',
  twenty_sessions: '20 sezení',
  streak_3: '3 dny v řadě',
  streak_7: '7 dní v řadě',
  streak_30: 'Měsíc v řadě',
  accuracy_95: '95 % přesnost',
  wpm_30: '30 WPM',
  wpm_40: '40 WPM',
  wpm_50: '50 WPM'
};
