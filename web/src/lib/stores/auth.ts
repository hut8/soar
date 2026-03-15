import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import type { User } from '$lib/types';
import { isTokenExpired } from '$lib/utils/jwt';

export interface AuthStore {
	isAuthenticated: boolean;
	user: User | null;
	token: string | null;
	isLoading: boolean;
}

const initialState: AuthStore = {
	isAuthenticated: false,
	user: null,
	token: null,
	isLoading: false
};

function createAuthStore() {
	const { subscribe, set, update } = writable<AuthStore>(initialState);

	// Listen for localStorage changes from other tabs or external clearing.
	// This ensures the auth store stays in sync if another tab logs out
	// or the browser evicts storage.
	if (browser) {
		window.addEventListener('storage', (event) => {
			// localStorage.clear() fires with key === null
			if (event.key === null || event.key === 'auth_token') {
				if (!event.newValue || event.key === null) {
					// Token was removed (logout in another tab or browser eviction)
					set(initialState);
				}
				// If a new token was set in another tab, we'd need the user data too.
				// For simplicity, just reload — the new tab's login set both values.
			}
		});
	}

	return {
		subscribe,
		login: (user: User, token: string) => {
			if (browser) {
				localStorage.setItem('auth_token', token);
				localStorage.setItem('auth_user', JSON.stringify(user));
			}
			set({
				isAuthenticated: true,
				user,
				token,
				isLoading: false
			});
		},
		logout: () => {
			if (browser) {
				localStorage.removeItem('auth_token');
				localStorage.removeItem('auth_user');
			}
			set(initialState);
		},
		setLoading: (loading: boolean) => {
			update((state) => ({ ...state, isLoading: loading }));
		},
		initFromStorage: () => {
			if (browser) {
				const token = localStorage.getItem('auth_token');
				const userStr = localStorage.getItem('auth_user');

				if (token && userStr) {
					// Reject expired tokens immediately rather than setting the store
					// to "authenticated" only to have the first API call trigger a logout.
					if (isTokenExpired(token)) {
						localStorage.removeItem('auth_token');
						localStorage.removeItem('auth_user');
						set(initialState);
						return;
					}

					try {
						const user = JSON.parse(userStr) as User;
						set({
							isAuthenticated: true,
							user,
							token,
							isLoading: false
						});
					} catch {
						// Clear invalid data
						localStorage.removeItem('auth_token');
						localStorage.removeItem('auth_user');
					}
				}
			}
		},
		updateUser: (user: User) => {
			if (browser) {
				localStorage.setItem('auth_user', JSON.stringify(user));
			}
			update((state) => {
				const prev = state.user;
				// Skip re-render if the fields that affect the UI haven't changed
				if (
					prev &&
					prev.id === user.id &&
					prev.clubId === user.clubId &&
					prev.isAdmin === user.isAdmin &&
					prev.isClubAdmin === user.isClubAdmin &&
					prev.firstName === user.firstName &&
					prev.lastName === user.lastName &&
					prev.email === user.email &&
					prev.emailVerified === user.emailVerified
				) {
					return state;
				}
				return { ...state, user };
			});
		}
	};
}

export const auth = createAuthStore();
