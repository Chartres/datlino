// Mock Tauri runtime so the SvelteKit SPA boots in a plain browser.
// Injected into every Playwright page via `addInitScript` BEFORE the SPA
// loads. Each invoke() command returns data shaped like the real Rust
// backend would produce, driven by an in-memory "corpus" the spec can
// swap per-persona.

import type { Page } from '@playwright/test';

export type MockCorpus = {
  documents: Array<{ id: number; source_path: string; kind: string; text: string }>;
  watchedRoots: string[];
  lessonsPassed: string[];
  sessionsCompleted: number;
  totalXp: number;
  streak: number;
  wpmBaseline: number | null;
  accuracyBaseline: number | null;
  anthropicKeyPresent: boolean;
  embeddingProvider: 'none' | 'fake' | 'cohere' | 'local';
};

export function defaultCorpus(overrides: Partial<MockCorpus> = {}): MockCorpus {
  return {
    documents: [],
    watchedRoots: [],
    lessonsPassed: [],
    sessionsCompleted: 0,
    totalXp: 0,
    streak: 0,
    wpmBaseline: null,
    accuracyBaseline: null,
    anthropicKeyPresent: false,
    embeddingProvider: 'fake',
    ...overrides
  };
}

export async function installMockTauri(page: Page, corpus: MockCorpus) {
  await page.addInitScript((initial) => {
    const state = JSON.parse(JSON.stringify(initial));

    // --- Helpers -------------------------------------------------------

    const segmentText = (text: string): string[] =>
      text
        // very rough CZ-aware segmentation; good enough for smoke tests.
        .split(/(?<=[\.\!\?])\s+/)
        .map((s) => s.trim())
        .filter((s) => s.length >= 12);

    const introLessons = [
      { id: 'home_row_left', title: 'Horní řada levá', subtitle: 'Prsty na asdf.', target_accuracy: 92, target_wpm: 10 },
      { id: 'home_row_right', title: 'Horní řada pravá', subtitle: 'jkl;', target_accuracy: 92, target_wpm: 10 },
      { id: 'home_row_both', title: 'Obě ruce, horní řada', subtitle: 'asdf jkl;', target_accuracy: 92, target_wpm: 12 },
      { id: 'top_row_left', title: 'Vrchní řada levá', subtitle: 'qwer', target_accuracy: 90, target_wpm: 14 },
      { id: 'top_row_right', title: 'Vrchní řada pravá', subtitle: 'uiop', target_accuracy: 90, target_wpm: 14 },
      { id: 'diacritics_hacek', title: 'Háčky', subtitle: 'č š ž ř ě', target_accuracy: 88, target_wpm: 16 },
      { id: 'short_sentences', title: 'Krátké věty', subtitle: 'Smysluplné věty.', target_accuracy: 90, target_wpm: 22 }
    ];

    const lessonDrills: Record<string, string[]> = {
      home_row_left: ['aaa sss ddd fff', 'asdf asdf', 'sad dad fad', 'asdf fdsa'],
      home_row_right: ['jjj kkk lll ;;;', 'jkl; jkl;', 'jill kill lull', 'lkjh'],
      home_row_both: ['asdf jkl;', 'ask a dad', 'a fad lad', 'asd fgh jkl'],
      top_row_left: ['qwer qwer', 'were read', 'free rates'],
      top_row_right: ['uiop uiop', 'loop pool', 'pour oil'],
      diacritics_hacek: ['čč šš žž řř ěě', 'čaj šálek žena', 'tři dřeva'],
      short_sentences: [
        'Dnes je krásný den.',
        'Učím se psát naslepo.',
        'Piš pomalu, ale přesně.'
      ]
    };

    // --- Tauri shim ----------------------------------------------------

    const invoke = async (cmd: string, args?: any) => {
      (window as any).__MOCK_CALLS__ = (window as any).__MOCK_CALLS__ || [];
      (window as any).__MOCK_CALLS__.push({ cmd, args });

      switch (cmd) {
        case 'index_status':
          return {
            document_count: state.documents.length,
            chunk_count: state.documents.reduce(
              (acc: number, d: any) => acc + segmentText(d.text).length,
              0
            ),
            watched_roots: state.watchedRoots
          };

        case 'add_watched_folder':
          if (!state.watchedRoots.includes(args.path))
            state.watchedRoots.push(args.path);
          return;

        case 'remove_watched_folder':
          state.watchedRoots = state.watchedRoots.filter((p: string) => p !== args.path);
          return;

        case 'search_chunks': {
          const q = (args.query as string).toLowerCase();
          const hits: any[] = [];
          for (const doc of state.documents) {
            segmentText(doc.text).forEach((s, i) => {
              if (s.toLowerCase().includes(q)) {
                hits.push({
                  chunk_id: doc.id * 1000 + i,
                  document_id: doc.id,
                  source_path: doc.source_path,
                  text: s,
                  char_offset: i,
                  score: 1.0 / (i + 1)
                });
              }
            });
          }
          return hits.slice(0, args.k ?? 10);
        }

        case 'list_documents':
          return state.documents.map((d: any) => ({
            id: d.id,
            source_path: d.source_path,
            kind: d.kind,
            chunk_count: segmentText(d.text).length
          }));

        case 'ingest_single_file':
          // Playwright harness doesn't read real files; tests push docs
          // into state directly via page.evaluate if they need them.
          return true;

        case 'create_session': {
          const req = args.request;
          let sentences: any[] = [];

          if (req.mode === 'intro_lesson') {
            const id = req.lesson_id || introLessons.find((l) => !state.lessonsPassed.includes(l.id))?.id || 'home_row_left';
            sentences = (lessonDrills[id] || []).map((text) => ({
              chunk_id: null,
              text,
              source_path: `lesson://${id}`,
              is_generated: true
            }));
          } else if (req.mode === 'content' && req.document_id) {
            const doc = state.documents.find((d: any) => d.id === req.document_id);
            if (doc) {
              sentences = segmentText(doc.text).map((text, i) => ({
                chunk_id: doc.id * 1000 + i,
                text,
                source_path: doc.source_path,
                is_generated: false
              }));
            }
          } else if (req.mode === 'content' && req.query) {
            const q = (req.query as string).toLowerCase();
            const matching = state.documents.filter((d: any) =>
              d.source_path.toLowerCase().includes(q) ||
              d.text.toLowerCase().includes(q)
            );
            for (const doc of matching.slice(0, 2)) {
              for (const [i, text] of segmentText(doc.text).entries()) {
                sentences.push({
                  chunk_id: doc.id * 1000 + i,
                  text,
                  source_path: doc.source_path,
                  is_generated: false
                });
              }
            }
          } else if (req.mode === 'diacritics') {
            sentences = [
              { chunk_id: null, text: 'čč aa čč aa', source_path: null, is_generated: true },
              { chunk_id: null, text: 'áč čá áš šá', source_path: null, is_generated: true }
            ];
          } else if (req.mode === 'warmup') {
            sentences = [
              { chunk_id: null, text: 'ahoj svete', source_path: null, is_generated: true },
              { chunk_id: null, text: 'piseme pomalu a presne', source_path: null, is_generated: true }
            ];
          }
          return {
            session_id: Math.floor(Math.random() * 1e6),
            mode: req.mode,
            alpha: req.alpha,
            sentences
          };
        }

        case 'finalize_session': {
          state.sessionsCompleted += 1;
          state.totalXp += 25;
          state.streak += 1;
          state.wpmBaseline = state.wpmBaseline ? state.wpmBaseline * 0.7 + 35 * 0.3 : 35;
          state.accuracyBaseline = state.accuracyBaseline ? state.accuracyBaseline * 0.7 + 95 * 0.3 : 95;
          const badges: string[] = [];
          if (state.sessionsCompleted === 1) badges.push('first_session');
          if (state.streak === 3) badges.push('streak_3');
          return {
            session_id: args.sessionId,
            wpm: 35,
            accuracy_pct: 95,
            xp_earned: 25,
            total_xp: state.totalXp,
            level: Math.floor(Math.sqrt(state.totalXp / 10)),
            current_streak: state.streak,
            longest_streak: state.streak,
            words_typed: 30,
            characters_typed: 150,
            sentences_completed: 3,
            sentences_attempted: 3,
            badges_awarded: badges,
            weak_preview: [
              { ngram: 'řř', occurrences: 4, ema_latency_ms: 420, ema_error_rate: 0.18, weakness: 0.7 }
            ]
          };
        }

        case 'get_profile':
          return {
            total_xp: state.totalXp,
            level: Math.floor(Math.sqrt(state.totalXp / 10)),
            current_streak: state.streak,
            longest_streak: state.streak,
            total_sessions: state.sessionsCompleted,
            wpm_baseline: state.wpmBaseline,
            accuracy_baseline: state.accuracyBaseline,
            badges: state.streak >= 3 ? ['first_session', 'streak_3'] : state.sessionsCompleted > 0 ? ['first_session'] : []
          };

        case 'get_history':
          return Array.from({ length: Math.min(state.sessionsCompleted, 5) }, (_, i) => ({
            session_id: i + 1,
            created_at: Date.now() / 1000 - i * 86400,
            mode: 'intro_lesson',
            alpha: 0,
            xp_earned: 25,
            summary: { wpm: 30 + i * 2, accuracy_pct: 92 + i, xp_earned: 25 }
          }));

        case 'get_weak_ngrams':
          return state.sessionsCompleted > 0
            ? [
                { ngram: 'řř', occurrences: 8, ema_latency_ms: 420, ema_error_rate: 0.18, weakness: 0.72 },
                { ngram: 'čš', occurrences: 5, ema_latency_ms: 380, ema_error_rate: 0.12, weakness: 0.55 }
              ]
            : [];

        case 'list_chapters':
          return [];

        case 'list_intro_lessons': {
          const passed = new Set(state.lessonsPassed);
          let prevPassed = true;
          return introLessons.map((l) => {
            const item = {
              id: l.id,
              title: l.title,
              subtitle: l.subtitle,
              target_accuracy: l.target_accuracy,
              target_wpm: l.target_wpm,
              unlocked: prevPassed,
              passed: passed.has(l.id),
              best_wpm: passed.has(l.id) ? 25 : 0,
              best_accuracy: passed.has(l.id) ? 95 : 0,
              attempts: passed.has(l.id) ? 1 : 0
            };
            prevPassed = passed.has(l.id);
            return item;
          });
        }

        case 'get_embedding_status':
          return {
            provider: state.embeddingProvider,
            dim: state.embeddingProvider === 'fake' ? 256 : state.embeddingProvider === 'cohere' ? 1024 : 384,
            embedded_chunks: state.documents.length * 3,
            total_chunks: state.documents.length * 5,
            cohere_key_present: state.embeddingProvider === 'cohere'
          };

        case 'set_embedding_provider':
          state.embeddingProvider = args.kind;
          return { embedded: 10, total_chunks: 10 };

        case 'embed_pending':
          return { embedded: 5, total_chunks: 10 };

        case 'set_cohere_api_key':
          return;

        case 'get_ocr_status':
          // The mock says "missing" by default so Tereza's persona can
          // exercise the install-hint flow.
          return { tesseract: false, pdftoppm: false, available: false };

        case 'set_anthropic_api_key':
          state.anthropicKeyPresent = (args.key ?? '').length > 0;
          return;

        case 'anthropic_key_present':
          return state.anthropicKeyPresent;

        case 'detect_anthropic_env_key':
          return null;

        case 'get_changelog':
          // A short excerpt is enough for the spec to assert structure.
          // In production this comes from include_str! on the real file.
          return `# Datlino — Changelog\n\n## Unreleased\n\n## 2026-04-24\n\n**IA reorg: two doors instead of six modes.**\n\n- Home screen is now a dashboard.\n- /learn + /study as the two top-level paths.\n\n## 2026-04-23\n\n**Dead-key diacritics fixed for all layouts.**\n\n- \`ř\`, \`č\`, \`š\`, \`ž\`, \`ě\`, \`ů\` register as ONE keystroke.\n`;

        case 'get_version':
          return '0.1.0-mock';

        // Dialog plugin — tauri-apps/plugin-dialog routes its open() call
        // through __TAURI_INTERNALS__.invoke('plugin:dialog|open', …).
        case 'plugin:dialog|open': {
          const opts = args?.options ?? args ?? {};
          if (opts.directory) return '/mock/study-folder';
          return '/mock/study-folder/chemie.md';
        }

        default:
          // eslint-disable-next-line no-console
          console.warn('[mock-tauri] unhandled command:', cmd, args);
          return null;
      }
    };

    (window as any).__TAURI_INTERNALS__ = {
      invoke,
      transformCallback: (cb: any) => cb,
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } }
    };

    // Mock the dialog plugin too. The real one opens a native picker;
    // in the browser we just resolve with a canned path for tests.
    (window as any).__MOCK_DIALOG__ = {
      pickFolder: '/mock/study-folder',
      pickFile: '/mock/study-folder/chemie.md'
    };

    (window as any).__MOCK_STATE__ = state;
  }, corpus);
}
