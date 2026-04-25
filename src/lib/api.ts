import { invoke } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import type {
  AttemptRecord,
  ChapterInfo,
  ClaudeSubscriptionStatus,
  DocumentInfo,
  EmbedProgress,
  EmbeddingProviderKind,
  EmbeddingStatus,
  ExamRamp,
  IndexStatus,
  LessonListItem,
  SearchHit,
  SessionHistoryRow,
  SessionPlan,
  SessionRequest,
  SessionSummary,
  UserProfileView,
  WeakNgram
} from './types';

export const api = {
  // library
  indexStatus: () => invoke<IndexStatus>('index_status'),
  addWatchedFolder: (path: string) =>
    invoke<void>('add_watched_folder', { path }),
  removeWatchedFolder: (path: string) =>
    invoke<void>('remove_watched_folder', { path }),
  searchChunks: (query: string, k = 10) =>
    invoke<SearchHit[]>('search_chunks', { query, k }),

  // sessions
  createSession: (request: SessionRequest) =>
    invoke<SessionPlan>('create_session', { request }),
  finalizeSession: (sessionId: number, attempts: AttemptRecord[]) =>
    invoke<SessionSummary>('finalize_session', {
      sessionId,
      attempts
    }),

  // profile / history
  getProfile: () => invoke<UserProfileView>('get_profile'),
  getHistory: (limit = 20) => invoke<SessionHistoryRow[]>('get_history', { limit }),
  getWeakNgrams: (limit = 20) =>
    invoke<WeakNgram[]>('get_weak_ngrams', { limit }),
  listChapters: () => invoke<ChapterInfo[]>('list_chapters'),

  // embeddings / settings
  getEmbeddingStatus: () => invoke<EmbeddingStatus>('get_embedding_status'),
  setCohereApiKey: (key: string) => invoke<void>('set_cohere_api_key', { key }),
  setEmbeddingProvider: (kind: EmbeddingProviderKind) =>
    invoke<EmbedProgress>('set_embedding_provider', { kind }),
  embedPending: () => invoke<EmbedProgress>('embed_pending'),

  // OCR + rephrase
  getOcrStatus: () =>
    invoke<{ tesseract: boolean; pdftoppm: boolean; available: boolean }>(
      'get_ocr_status'
    ),
  setAnthropicApiKey: (key: string) =>
    invoke<void>('set_anthropic_api_key', { key }),
  anthropicKeyPresent: () => invoke<boolean>('anthropic_key_present'),
  detectAnthropicEnvKey: () => invoke<string | null>('detect_anthropic_env_key'),
  claudeSubscriptionStatus: () =>
    invoke<ClaudeSubscriptionStatus>('claude_subscription_status'),

  // intro lessons
  listIntroLessons: () => invoke<LessonListItem[]>('list_intro_lessons'),
  listExamRamps: () => invoke<ExamRamp[]>('list_exam_ramps'),

  // copy-paste rephrase
  buildCopyPastePrompt: (
    sources: string[],
    weakNgrams: string[],
    language?: string,
    style?: 'keystrokes' | 'thing_explainer' | 'both'
  ) =>
    invoke<{ prompt: string; expected_count: number }>('build_copy_paste_prompt', {
      args: { sources, weak_ngrams: weakNgrams, language, style }
    }),
  applyCopyPasteRephrase: (sources: string[], raw: string, similarityFloor?: number) =>
    invoke<{
      outcomes: {
        text: string;
        similarity: number;
        generator_model: string;
        accepted: boolean;
      }[];
      warnings: string[];
    }>('apply_copy_paste_rephrase', {
      args: { sources, raw, similarity_floor: similarityFloor }
    }),

  // about + changelog
  getChangelog: () => invoke<string>('get_changelog'),
  getVersion: () => invoke<string>('get_version'),

  // calibration + metacognition
  recordCalibrationPrediction: (sessionId: number, predictedAccuracyPct: number) =>
    invoke<void>('record_calibration_prediction', {
      sessionId,
      predictedAccuracyPct
    }),
  recordCalibrationReflection: (
    sessionId: number,
    actualAccuracyPct: number,
    difficulty: number | null,
    note: string | null
  ) =>
    invoke<void>('record_calibration_reflection', {
      sessionId,
      actualAccuracyPct,
      difficulty,
      note
    }),
  calibrationHistory: (limit = 30) =>
    invoke<
      {
        session_id: number;
        predicted: number;
        actual: number | null;
        difficulty: number | null;
        created_at: number;
      }[]
    >('calibration_history', { limit }),

  // documents / file picker
  listDocuments: () => invoke<DocumentInfo[]>('list_documents'),
  ingestSingleFile: (path: string) => invoke<boolean>('ingest_single_file', { path }),

  // native dialog
  pickFolder: async (): Promise<string | null> => {
    const result = await openDialog({
      directory: true,
      multiple: false,
      title: 'Vyber složku s poznámkami'
    });
    return typeof result === 'string' ? result : null;
  },
  pickFile: async (): Promise<string | null> => {
    const result = await openDialog({
      directory: false,
      multiple: false,
      title: 'Vyber soubor k importu',
      filters: [
        { name: 'Poznámky', extensions: ['md', 'markdown', 'txt', 'pdf'] }
      ]
    });
    return typeof result === 'string' ? result : null;
  }
};
