import { writable } from 'svelte/store';

interface LoadingState {
	activeRequests: number;
}

function createLoadingStore() {
	const { subscribe, update } = writable<LoadingState>({
		activeRequests: 0
	});

	return {
		subscribe,
		startRequest: () =>
			update((state) => ({
				activeRequests: state.activeRequests + 1
			})),
		endRequest: () =>
			update((state) => ({
				activeRequests: Math.max(0, state.activeRequests - 1)
			})),
		reset: () =>
			update(() => ({
				activeRequests: 0
			}))
	};
}

export const loading = createLoadingStore();
