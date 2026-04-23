// Finger-zone keyboard layout used by the on-screen keyboard.
//
// Based on the canonical touch-typing finger assignments. We render a
// US QWERTY layout as the baseline; the Czech-specific diacritics live
// in a separate Czech-programmers' row and the component maps a target
// character to the right physical key + modifier.

export type Finger =
  | 'l-pinky'
  | 'l-ring'
  | 'l-middle'
  | 'l-index'
  | 'r-index'
  | 'r-middle'
  | 'r-ring'
  | 'r-pinky'
  | 'thumb';

export type KeyDef = {
  /** What the key produces unshifted. */
  base: string;
  /** What the key produces with Shift. */
  shift?: string;
  /** Display label (overrides base when set). */
  label?: string;
  /** Finger that presses this key in the touch-typing system. */
  finger: Finger;
  /** Visual width in grid units (1 = standard key). */
  width?: number;
  /** Mark home-row keys so we can dot them. */
  homeDot?: boolean;
};

// US QWERTY physical layout, four rows + spacebar.
export const US_LAYOUT: KeyDef[][] = [
  // Number row
  [
    { base: '`', shift: '~', finger: 'l-pinky' },
    { base: '1', shift: '!', finger: 'l-pinky' },
    { base: '2', shift: '@', finger: 'l-ring' },
    { base: '3', shift: '#', finger: 'l-middle' },
    { base: '4', shift: '$', finger: 'l-index' },
    { base: '5', shift: '%', finger: 'l-index' },
    { base: '6', shift: '^', finger: 'r-index' },
    { base: '7', shift: '&', finger: 'r-index' },
    { base: '8', shift: '*', finger: 'r-middle' },
    { base: '9', shift: '(', finger: 'r-ring' },
    { base: '0', shift: ')', finger: 'r-pinky' },
    { base: '-', shift: '_', finger: 'r-pinky' },
    { base: '=', shift: '+', finger: 'r-pinky' },
    { base: 'Backspace', label: '⌫', finger: 'r-pinky', width: 1.8 }
  ],
  // Top row
  [
    { base: 'Tab', label: '⇥', finger: 'l-pinky', width: 1.5 },
    { base: 'q', shift: 'Q', finger: 'l-pinky' },
    { base: 'w', shift: 'W', finger: 'l-ring' },
    { base: 'e', shift: 'E', finger: 'l-middle' },
    { base: 'r', shift: 'R', finger: 'l-index' },
    { base: 't', shift: 'T', finger: 'l-index' },
    { base: 'y', shift: 'Y', finger: 'r-index' },
    { base: 'u', shift: 'U', finger: 'r-index' },
    { base: 'i', shift: 'I', finger: 'r-middle' },
    { base: 'o', shift: 'O', finger: 'r-ring' },
    { base: 'p', shift: 'P', finger: 'r-pinky' },
    { base: '[', shift: '{', finger: 'r-pinky' },
    { base: ']', shift: '}', finger: 'r-pinky' },
    { base: '\\', shift: '|', finger: 'r-pinky', width: 1.3 }
  ],
  // Home row
  [
    { base: 'CapsLock', label: '⇪', finger: 'l-pinky', width: 1.8 },
    { base: 'a', shift: 'A', finger: 'l-pinky', homeDot: true },
    { base: 's', shift: 'S', finger: 'l-ring' },
    { base: 'd', shift: 'D', finger: 'l-middle' },
    { base: 'f', shift: 'F', finger: 'l-index', homeDot: true },
    { base: 'g', shift: 'G', finger: 'l-index' },
    { base: 'h', shift: 'H', finger: 'r-index' },
    { base: 'j', shift: 'J', finger: 'r-index', homeDot: true },
    { base: 'k', shift: 'K', finger: 'r-middle' },
    { base: 'l', shift: 'L', finger: 'r-ring' },
    { base: ';', shift: ':', finger: 'r-pinky', homeDot: true },
    { base: "'", shift: '"', finger: 'r-pinky' },
    { base: 'Enter', label: '⏎', finger: 'r-pinky', width: 2.3 }
  ],
  // Bottom row
  [
    { base: 'Shift-L', label: '⇧', finger: 'l-pinky', width: 2.3 },
    { base: 'z', shift: 'Z', finger: 'l-pinky' },
    { base: 'x', shift: 'X', finger: 'l-ring' },
    { base: 'c', shift: 'C', finger: 'l-middle' },
    { base: 'v', shift: 'V', finger: 'l-index' },
    { base: 'b', shift: 'B', finger: 'l-index' },
    { base: 'n', shift: 'N', finger: 'r-index' },
    { base: 'm', shift: 'M', finger: 'r-index' },
    { base: ',', shift: '<', finger: 'r-middle' },
    { base: '.', shift: '>', finger: 'r-ring' },
    { base: '/', shift: '?', finger: 'r-pinky' },
    { base: 'Shift-R', label: '⇧', finger: 'r-pinky', width: 2.5 }
  ],
  // Space row
  [
    { base: ' ', label: 'mezera', finger: 'thumb', width: 8 }
  ]
];

// Czech dead keys (háček / acute) — on CZ QWERTZ these live on the number
// row; here we map the output diacritic character back to the physical key
// + the required modifier combo, so the component can highlight correctly
// on a US keyboard with Czech input method.
//
// The approach: for each diacritic char, produce { shift, deadKey, key }
// where `deadKey` is a side-note ("press [ then a") shown under the main
// highlight. Good enough for a hint; a future increment can do a full
// CZ QWERTZ layout render.
export const CZ_DEAD_KEY_MAP: Record<string, { via: string; then: string }> = {
  // Háček — dead key at `[` on common CZ programmers layouts
  č: { via: 'AltGr+;', then: 'c' },
  š: { via: 'AltGr+;', then: 's' },
  ž: { via: 'AltGr+;', then: 'z' },
  ř: { via: 'AltGr+;', then: 'r' },
  ě: { via: 'AltGr+;', then: 'e' },
  ň: { via: 'AltGr+;', then: 'n' },
  ď: { via: 'AltGr+;', then: 'd' },
  ť: { via: 'AltGr+;', then: 't' },
  // Acute — dead key at `\'` on common CZ programmers layouts
  á: { via: "AltGr+'", then: 'a' },
  é: { via: "AltGr+'", then: 'e' },
  í: { via: "AltGr+'", then: 'i' },
  ó: { via: "AltGr+'", then: 'o' },
  ú: { via: "AltGr+'", then: 'u' },
  ý: { via: "AltGr+'", then: 'y' },
  ů: { via: 'AltGr+[', then: 'u' }
};

/** Resolve a target character to the physical key the student should press. */
export function keyForChar(ch: string): {
  base: string;
  shift: boolean;
  dead?: { via: string; then: string };
} {
  if (ch === ' ') return { base: ' ', shift: false };
  const lower = ch.toLowerCase();
  // Czech diacritic?
  if (lower in CZ_DEAD_KEY_MAP) {
    const map = CZ_DEAD_KEY_MAP[lower];
    return {
      base: map.then,
      shift: ch !== lower,
      dead: map
    };
  }
  // Regular ASCII: scan rows for a matching base or shift.
  for (const row of US_LAYOUT) {
    for (const key of row) {
      if (key.base === lower) return { base: lower, shift: ch !== lower };
      if (key.shift === ch) return { base: key.base, shift: true };
    }
  }
  return { base: ch, shift: false };
}

export const FINGER_COLORS: Record<Finger, string> = {
  'l-pinky': '#ef4444', // red
  'l-ring': '#f59e0b', // amber
  'l-middle': '#eab308', // yellow
  'l-index': '#22c55e', // green
  'r-index': '#14b8a6', // teal
  'r-middle': '#3b82f6', // blue
  'r-ring': '#a855f7', // purple
  'r-pinky': '#ec4899', // pink
  thumb: '#78716c' // stone
};

export const FINGER_LABELS: Record<Finger, string> = {
  'l-pinky': 'L malíček',
  'l-ring': 'L prsteníček',
  'l-middle': 'L prostředníček',
  'l-index': 'L ukazováček',
  'r-index': 'P ukazováček',
  'r-middle': 'P prostředníček',
  'r-ring': 'P prsteníček',
  'r-pinky': 'P malíček',
  thumb: 'Palec'
};
