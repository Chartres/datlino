import { api } from './api';
import type { UserProfileView } from './types';

// Shared reactive profile store (Svelte 5 runes). Pages import `profile`
// and mutate it by calling `refresh()` after actions that change server
// state (finalising a session, etc).
export const profile = $state<{ data: UserProfileView | null; loading: boolean }>(
  {
    data: null,
    loading: false
  }
);

export async function refreshProfile() {
  profile.loading = true;
  try {
    profile.data = await api.getProfile();
  } finally {
    profile.loading = false;
  }
}
