import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		// A single page app: nginx serves index.html for every route and the
		// browser talks to the API under /api.
		adapter: adapter({ fallback: 'index.html' })
	}
};

export default config;
