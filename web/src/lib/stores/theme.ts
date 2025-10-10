import { writable } from 'svelte/store';
import { browser } from '$app/environment';

type Theme = 'light' | 'dark';

function createThemeStore() {
	const { subscribe, set, update } = writable<Theme>('light');

	return {
		subscribe,
		// Initialize theme from localStorage or system preference
		init: () => {
			if (browser) {
				const stored = localStorage.getItem('theme') as Theme | null;
				const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;

				if (stored) {
					set(stored);
					applyTheme(stored);
				} else if (systemPrefersDark) {
					set('dark');
					applyTheme('dark');
				} else {
					set('light');
					applyTheme('light');
				}
			}
		},
		// Toggle between light and dark
		toggle: () => {
			update((current) => {
				const newTheme = current === 'light' ? 'dark' : 'light';
				applyTheme(newTheme);
				if (browser) {
					localStorage.setItem('theme', newTheme);
				}
				return newTheme;
			});
		},
		// Set specific theme
		setTheme: (theme: Theme) => {
			set(theme);
			applyTheme(theme);
			if (browser) {
				localStorage.setItem('theme', theme);
			}
		}
	};
}

function applyTheme(theme: Theme) {
	if (browser) {
		if (theme === 'dark') {
			document.documentElement.classList.add('dark');
		} else {
			document.documentElement.classList.remove('dark');
		}
	}
}

export const theme = createThemeStore();
