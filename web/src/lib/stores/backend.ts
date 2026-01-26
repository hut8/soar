import { writable } from 'svelte/store';
import { browser } from '$app/environment';

export type BackendMode = 'dev' | 'staging' | 'prod';

// Labels for display in the UI
export const BACKEND_LABELS: Record<BackendMode, string> = {
	dev: 'Development',
	staging: 'Staging',
	prod: 'Production'
};

// Short labels for compact display
export const BACKEND_SHORT_LABELS: Record<BackendMode, string> = {
	dev: 'D',
	staging: 'S',
	prod: 'P'
};

function createBackendStore() {
	const { subscribe, set } = writable<BackendMode>('staging');

	return {
		subscribe,
		// Initialize backend mode from localStorage, defaulting to staging
		init: () => {
			if (browser) {
				const stored = localStorage.getItem('backendMode') as BackendMode | null;
				set(stored || 'staging');
			}
		},
		// Set specific backend mode and reload
		setMode: (mode: BackendMode) => {
			set(mode);
			if (browser) {
				localStorage.setItem('backendMode', mode);
				// Reload the page to apply the new backend setting
				window.location.reload();
			}
		}
	};
}

export const backendMode = createBackendStore();
