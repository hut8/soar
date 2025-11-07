export default {
	plugins: {
		'postcss-import': {
			resolve(id) {
				// Resolve @skeletonlabs packages to their CSS exports
				if (id === '@skeletonlabs/skeleton-common') {
					return '@skeletonlabs/skeleton-common/dist/index.css';
				}
				if (id === '@skeletonlabs/skeleton') {
					return '@skeletonlabs/skeleton/dist/index.css';
				}
				if (id === '@skeletonlabs/skeleton-svelte') {
					return '@skeletonlabs/skeleton-svelte/dist/index.css';
				}
				return id;
			}
		},
		'@tailwindcss/postcss': {},
		autoprefixer: {}
	}
};
