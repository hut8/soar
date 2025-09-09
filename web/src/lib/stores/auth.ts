import { writable } from 'svelte/store';
import { browser } from '$app/environment';

export interface User {
	id: string;
	first_name: string;
	last_name: string;
	email: string;
	access_level: 'standard' | 'admin';
	club_id?: string;
	email_verified: boolean;
}

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
			update((state) => ({ ...state, user }));
		}
	};
}

export const auth = createAuthStore();
