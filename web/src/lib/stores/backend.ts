import { writable } from 'svelte/store';
import { browser } from '$app/environment';

export type BackendMode = 'dev' | 'prod';

function createBackendStore() {
	const { subscribe, set, update } = writable<BackendMode>('prod');

	return {
		subscribe,
		// Initialize backend mode from localStorage, defaulting to production
		init: () => {
			if (browser) {
				const stored = localStorage.getItem('backendMode') as BackendMode | null;
				// Default to production if not set
				set(stored || 'prod');
			}
		},
		// Toggle between dev and prod
		toggle: () => {
			update((current) => {
				const newMode = current === 'dev' ? 'prod' : 'dev';
				if (browser) {
					localStorage.setItem('backendMode', newMode);
					// Reload the page to apply the new backend setting
					window.location.reload();
				}
				return newMode;
			});
		},
		// Set specific backend mode
		setMode: (mode: BackendMode) => {
			set(mode);
			if (browser) {
				localStorage.setItem('backendMode', mode);
			}
		}
	};
}

export const backendMode = createBackendStore();
