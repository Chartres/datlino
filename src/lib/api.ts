import { invoke } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import type {
  AttemptRecord,
  ChapterInfo,
  IndexStatus,
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

  // native dialog
  pickFolder: async (): Promise<string | null> => {
    const result = await openDialog({
      directory: true,
      multiple: false,
      title: 'Vyber složku s poznámkami'
    });
    return typeof result === 'string' ? result : null;
  }
};
