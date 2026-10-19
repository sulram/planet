// Shared by every script in this folder.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

export const ROOT = fileURLToPath(new URL('..', import.meta.url)).replace(/\/$/, '');

/** A plugin this version carries, as `plugins.json` lists it. */
export interface Listed {
	name: string;
	/** Whether a world starts with it on (DECISIONS 91). */
	on: boolean;
}

/** The version's plugins, in the order the config lists them. */
export function plugins(): Listed[] {
	return JSON.parse(readFileSync(`${ROOT}/plugins.json`, 'utf8')) as Listed[];
}

/** Where the world server and the page listen in development. */
export const HOST = '127.0.0.1';

/**
 * `preferred` when nothing holds it on `HOST`, else the first port above it
 * that is free, as Vite picks the page's: several checkouts run `bun run dev`
 * side by side, each with its own world and page, on ports one can remember.
 */
export function freePort(preferred: number): number {
	for (let port = preferred; port < preferred + 100; port++) {
		try {
			Bun.serve({ hostname: HOST, port, fetch: () => new Response() }).stop(true);
			return port;
		} catch (error) {
			if ((error as { code?: string }).code !== 'EADDRINUSE') throw error;
		}
	}
	throw new Error(`no free port from ${preferred} on`);
}
