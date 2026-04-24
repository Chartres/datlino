// Helper to record qualitative observations during a persona run.
// Each entry ends up in `research/interviews/<persona>.md` so findings
// are grep-able.

import { appendFileSync, mkdirSync } from 'fs';
import { join, dirname } from 'path';

export type Observation = {
  persona: string;
  moment: string; // short label, e.g. "first-screen"
  note: string;
  severity?: 'info' | 'confusion' | 'blocker' | 'delight';
};

export function record(obs: Observation) {
  const outPath = join(
    process.cwd(),
    'research',
    'interviews',
    `${obs.persona}.md`
  );
  mkdirSync(dirname(outPath), { recursive: true });
  const line = `- **[${obs.moment}]** ${obs.severity ? `_(${obs.severity})_ ` : ''}${obs.note}\n`;
  appendFileSync(outPath, line);
}

export function openFile(persona: string, voice: string) {
  const outPath = join(
    process.cwd(),
    'research',
    'interviews',
    `${persona}.md`
  );
  mkdirSync(dirname(outPath), { recursive: true });
  appendFileSync(outPath, `\n## Session ${new Date().toISOString()}\n\n> ${voice}\n\n`);
}
