import tailwindcss from '@tailwindcss/vite';
import devtoolsJson from 'vite-plugin-devtools-json';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit(), devtoolsJson()],
	// Proxy /data/* requests to the Rust backend for E2E tests and local development
	server: {
		proxy: {
			'/data': {
				target: 'http://localhost:61226',
				changeOrigin: true
			}
		}
	},
	preview: {
		proxy: {
			'/data': {
				target: 'http://localhost:61226',
				changeOrigin: true
			}
		}
	}
});
