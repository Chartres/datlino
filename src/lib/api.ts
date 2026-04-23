import { invoke } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import type {
  AttemptRecord,
  ChapterInfo,
  DocumentInfo,
  EmbedProgress,
  EmbeddingProviderKind,
  EmbeddingStatus,
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

  // intro lessons
  listIntroLessons: () => invoke<LessonListItem[]>('list_intro_lessons'),

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
