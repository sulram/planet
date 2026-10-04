import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

/** Where the world server listens in development (`scripts/server.ts`). */
const WORLD = 'http://127.0.0.1:8090';

export default defineConfig({
	plugins: [
		sveltekit({
			compilerOptions: {
				// Runes mode for every project file; libraries keep their own mode.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},
			// The page is only files, and the world server sends them
			// (DECISIONS 90). An address that is no route gets `404.html`,
			// which draws the error page.
			adapter: adapter({ fallback: '404.html' })
		})
	],
	server: {
		// One origin in development too: Vite holds the page and hands the
		// world's routes and its socket to the Go server.
		proxy: { '/api': { target: WORLD, ws: true } },
		// A plugin's panel lives in its own folder, outside this app
		// (DECISIONS 100).
		fs: { allow: ['../../plugins'] }
	}
});
