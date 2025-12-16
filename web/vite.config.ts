import tailwindcss from '@tailwindcss/vite';
import devtoolsJson from 'vite-plugin-devtools-json';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { viteStaticCopy } from 'vite-plugin-static-copy';
import externalGlobals from 'rollup-plugin-external-globals';

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
		})
	],
	// Define CESIUM_BASE_URL for the application
	define: {
		CESIUM_BASE_URL: JSON.stringify('/cesium/')
	},
	// Configure build to externalize cesium and use the global Cesium object
	build: {
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
	}
});
