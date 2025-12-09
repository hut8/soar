import tailwindcss from '@tailwindcss/vite';
import devtoolsJson from 'vite-plugin-devtools-json';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit(), devtoolsJson()],
	// Proxy /data/* requests to the Rust backend for local development
	server: {
		proxy: {
			'/data': {
				target: 'http://localhost:61225',
				changeOrigin: true
			}
		}
	}
});
