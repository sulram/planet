import type { Cookies, RequestEvent } from '@sveltejs/kit';
import { dev } from '$app/environment';

const COOKIE = 'avatar';

/** What `assets/manifest.json` says about avatars: the set this instance offers. */
async function offered(fetch: typeof globalThis.fetch): Promise<string[]> {
	try {
		const manifest: unknown = await (await fetch('/assets/manifest.json')).json();
		const avatars = (manifest as { avatars?: unknown }).avatars;
		return Array.isArray(avatars) ? avatars.filter((path) => typeof path === 'string') : [];
	} catch {
		return [];
	}
}

function keep(cookies: Cookies, reference: string) {
	cookies.set(COOKIE, reference, {
		path: '/',
		httpOnly: true,
		secure: !dev,
		sameSite: 'lax',
		maxAge: 60 * 60 * 24 * 365
	});
}

/**
 * The asset reference of the avatar this person wears. An avatar is always a
 * reference (a path under the asset root, or a URL), never an index, so a
 * person's own avatars will load exactly like the offered ones. Resolution:
 *
 * 1. the default avatar of the signed in user (arrives with user avatars);
 * 2. what this visitor chose before, kept in a cookie;
 * 3. a random avatar from the offered set, which then becomes their choice.
 *
 * Null when nothing is offered; the engine then wears its own default.
 */
export async function visitorAvatar({ cookies, fetch }: RequestEvent): Promise<string | null> {
	const set = await offered(fetch);
	const kept = cookies.get(COOKIE);
	if (kept && set.includes(kept)) return kept;
	if (set.length === 0) return null;

	const picked = set[crypto.getRandomValues(new Uint32Array(1))[0] % set.length];
	keep(cookies, picked);
	return picked;
}

/** Records a choice made in the world. Only offered avatars are accepted. */
export async function chooseAvatar({ cookies, fetch }: RequestEvent, reference: string): Promise<boolean> {
	if (!(await offered(fetch)).includes(reference)) return false;
	keep(cookies, reference);
	return true;
}
