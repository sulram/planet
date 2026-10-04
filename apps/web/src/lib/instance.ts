/**
 * The world's own routes, as a page asks them (`server/internal/api`). One
 * origin holds the page, the routes and the socket, so every address here is
 * a path.
 */
import type { Level } from '$lib/engine/protocol';
import type { Recipe } from '$lib/world';

/** What a world says it is. */
export interface About {
	/** The world's name in mundos. Empty where there is no mundos. */
	name: string;
	founded: boolean;
	/** Null while the world is unfounded. */
	recipe: Recipe | null;
	protocol: number;
	version: string;
	/** mundos's door, which every load passes through. Empty where there is no mundos. */
	door: string;
}

/** Who the server takes this page's visitor for. */
export interface Entered {
	level: Level;
	/** The account's name, as mundos signed it. Empty for a visitor. */
	name: string;
	/** Shown on the socket and on a founding. Empty for a visitor, who needs none. */
	key: string;
}

/** The world's socket, as the page hands it to the engine. */
export interface Link {
	/** The socket's address, the key in it. */
	url: string;
	/**
	 * Asked before the link is opened again after a drop. The page leaves
	 * through the door here when its key no longer stands.
	 */
	again?: () => Promise<void>;
}

/** A route's refusal: the server's code, or `unreachable` when it did not answer. */
export class Refused extends Error {
	constructor(readonly code: string) {
		super(code);
	}
}

async function ask<T>(path: string, init?: RequestInit): Promise<T> {
	let response: Response;
	try {
		response = await fetch(path, init);
	} catch {
		throw new Refused('unreachable');
	}
	const body: unknown = await response.json().catch(() => null);
	if (!response.ok) {
		const code = (body as { error?: unknown } | null)?.error;
		throw new Refused(typeof code === 'string' ? code : 'unreachable');
	}
	return body as T;
}

const bearer = (key: string): Record<string, string> => (key ? { authorization: `Bearer ${key}` } : {});

export const about = () => ask<About>('/api/world');

/** Trades what the door gave the page, a token or `guest`, for who the page is here. */
export const enter = (identity: string) =>
	ask<Entered>('/api/enter', { method: 'POST', body: JSON.stringify({ identity }) });

/** Freezes the recipe of an unfounded world. An admin's alone. */
export const found = (recipe: Recipe, key: string) =>
	ask<About>('/api/world', { method: 'POST', headers: bearer(key), body: JSON.stringify(recipe) });

/** Whether a key still stands. A server that does not answer is no verdict. */
export async function stands(key: string): Promise<boolean> {
	try {
		return (await fetch('/api/me', { headers: bearer(key) })).status !== 401;
	} catch {
		return true;
	}
}

/** The world socket's address, for this key. No key is a visitor. */
export function socket(key: string): string {
	const url = new URL('/api/socket', location.href);
	url.protocol = url.protocol.replace('http', 'ws');
	if (key) url.searchParams.set('key', key);
	return url.href;
}

/** Whether a level builds. Null is the offline preview, with no world to ask. */
export const builds = (level: Level | null) => level === null || level === 'builder' || level === 'admin';
