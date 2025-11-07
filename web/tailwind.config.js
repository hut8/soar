import { skeleton } from '@skeletonlabs/tw-plugin';

/** @type {import('tailwindcss').Config} */
export const darkMode = 'class';
export const content = [
	'./src/**/*.{html,js,svelte,ts}',
	'./node_modules/@skeletonlabs/skeleton-svelte/**/*.{html,js,svelte,ts}',
	'./node_modules/@skeletonlabs/skeleton-common/**/*.{html,js,svelte,ts}'
];
export const theme = {
	extend: {}
};
export const plugins = [
	skeleton({
		themes: {
			preset: [
				{
					name: 'skeleton',
					enhancements: true
				}
			]
		}
	})
];
