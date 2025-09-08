import { skeleton } from '@skeletonlabs/tw-plugin';
import { join } from 'path';

/** @type {import('tailwindcss').Config} */
export const darkMode = 'class';
export const content = [
	'./src/**/*.{html,js,svelte,ts}',
	join(require.resolve('@skeletonlabs/skeleton'), '../**/*.{html,js,svelte,ts}')
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
