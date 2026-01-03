import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

const apiTarget = process.env.API_URL ?? 'http://localhost:4407';

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		port: 5187,
		proxy: {
			'/api': apiTarget,
			'/healthz': apiTarget
		}
	},
	test: {
		include: ['src/**/*.test.ts'],
		environment: 'node'
	}
});
