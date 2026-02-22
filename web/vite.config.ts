import tailwindcss from '@tailwindcss/vite';
import devtoolsJson from 'vite-plugin-devtools-json';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { viteStaticCopy } from 'vite-plugin-static-copy';
import externalGlobals from 'rollup-plugin-external-globals';
import { sentryVitePlugin } from '@sentry/vite-plugin';

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit(),
		devtoolsJson(),
		viteStaticCopy({
			targets: [
				{
					src: 'node_modules/cesium/Build/Cesium/Workers/*',
					dest: 'cesium/Workers'
				},
				{
					src: 'node_modules/cesium/Build/Cesium/Assets/*',
					dest: 'cesium/Assets'
				},
				{
					src: 'node_modules/cesium/Build/Cesium/Widgets/*',
					dest: 'cesium/Widgets'
				},
				{
					src: 'node_modules/cesium/Build/Cesium/ThirdParty/*',
					dest: 'cesium/ThirdParty'
				},
				{
					src: 'node_modules/cesium/Build/Cesium/Cesium.js',
					dest: 'cesium'
				}
			]
		}),
		// Upload source maps to Sentry (only when SENTRY_AUTH_TOKEN is set)
		// Source maps are also served publicly for browser DevTools debugging
		sentryVitePlugin({
			org: process.env.SENTRY_ORG,
			project: process.env.SENTRY_PROJECT,
			authToken: process.env.SENTRY_AUTH_TOKEN,
			// Don't delete source maps after upload - we want them publicly accessible
			sourcemaps: {
				filesToDeleteAfterUpload: []
			},
			// Only upload if auth token is available (typically in CI)
			disable: !process.env.SENTRY_AUTH_TOKEN
		})
	],
	// Define CESIUM_BASE_URL for the application
	define: {
		CESIUM_BASE_URL: JSON.stringify('/cesium/')
	},
	// Configure build to externalize cesium and use the global Cesium object
	build: {
		// Enable source maps for staging/production debugging
		sourcemap: true,
		rollupOptions: {
			external: ['cesium'],
			plugins: [
				externalGlobals({
					cesium: 'Cesium'
				})
			]
		}
	},
	// Proxy /data/* requests to the Rust backend for local development
	server: {
		proxy: {
			'/data': {
				target: 'http://localhost:61225',
				changeOrigin: true
			}
		}
	},
	// Also proxy for preview mode (npm run preview) so E2E tests work
	preview: {
		proxy: {
			'/data': {
				target: 'http://localhost:61225',
				changeOrigin: true
			}
		}
	}
});
