export type PracticeMode =
  | 'content'
  | 'warmup'
  | 'diacritics'
  | 'weak_keys'
  | 'hybrid'
  | 'intro_lesson';

export type LessonListItem = {
  id: string;
  title: string;
  subtitle: string;
  target_accuracy: number;
  target_wpm: number;
  unlocked: boolean;
  passed: boolean;
  best_wpm: number;
  best_accuracy: number;
  attempts: number;
};

export type ContentStrategy = 'across' | 'chapter' | 'exam_prep';

export type RephraseStyle = 'keystrokes' | 'thing_explainer' | 'both';

export type SessionRequest = {
  mode: PracticeMode;
  alpha: number;
  target_duration_s: number;
  query?: string;
  pinned_source_prefixes?: string[];
  content_strategy?: ContentStrategy;
  chapter_id?: string;
  rephrase?: boolean;
  rephrase_style?: RephraseStyle;
  language?: string;
  lesson_id?: string;
  document_id?: number;
};

export type DocumentInfo = {
  id: number;
  source_path: string;
  kind: string;
  chunk_count: number;
};

export type ChapterInfo = {
  id: string;
  document_id: number;
  source_path: string;
  section: string;
  sentence_count: number;
};

export type SessionSentence = {
  chunk_id: number | null;
  text: string;
  source_path: string | null;
  is_generated: boolean;
  source_text?: string;
  rephrased_id?: number;
  similarity?: number;
};

export type SessionPlan = {
  session_id: number;
  mode: PracticeMode;
  alpha: number;
  sentences: SessionSentence[];
};

export type Keystroke = {
  t_ms: number;
  actual: string;
  expected: string;
  correct: boolean;
};

export type AttemptRecord = {
  chunk_id: number | null;
  target_text: string;
  started_at_ms: number;
  finished_at_ms: number;
  keystrokes: Keystroke[];
  completed: boolean;
};

export type WeakNgram = {
  ngram: string;
  occurrences: number;
  ema_latency_ms: number;
  ema_error_rate: number;
  weakness: number;
};

export type SessionSummary = {
  session_id: number;
  wpm: number;
  accuracy_pct: number;
  xp_earned: number;
  total_xp: number;
  level: number;
  current_streak: number;
  longest_streak: number;
  words_typed: number;
  characters_typed: number;
  sentences_completed: number;
  sentences_attempted: number;
  badges_awarded: string[];
  weak_preview: WeakNgram[];
};

export type UserProfileView = {
  total_xp: number;
  level: number;
  current_streak: number;
  longest_streak: number;
  total_sessions: number;
  wpm_baseline: number | null;
  accuracy_baseline: number | null;
  badges: string[];
};

export type SessionHistoryRow = {
  session_id: number;
  created_at: number;
  mode: string;
  alpha: number;
  xp_earned: number;
  summary: SessionSummary | null;
};

export type IndexStatus = {
  document_count: number;
  chunk_count: number;
  watched_roots: string[];
};

export type EmbeddingProviderKind = 'none' | 'fake' | 'cohere' | 'local';

export type EmbeddingStatus = {
  provider: EmbeddingProviderKind | string;
  dim: number;
  embedded_chunks: number;
  total_chunks: number;
  cohere_key_present: boolean;
};

export type EmbedProgress = {
  embedded: number;
  total_chunks: number;
};

export type SearchHit = {
  chunk_id: number;
  document_id: number;
  source_path: string;
  text: string;
  char_offset: number;
  score: number;
};
