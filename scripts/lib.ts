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
