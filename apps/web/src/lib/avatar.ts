import { keep, kept } from './kept';

/** What `assets/manifest.json` says about avatars: the set this version offers. */
async function offered(): Promise<string[]> {
	try {
		const manifest: unknown = await (await fetch('/assets/manifest.json')).json();
		const avatars = (manifest as { avatars?: unknown }).avatars;
		return Array.isArray(avatars) ? avatars.filter((path) => typeof path === 'string') : [];
	} catch {
		return [];
	}
}

/**
 * The asset reference of the avatar this person wears. An avatar is always a
 * reference (a path under the asset root, or a URL), never an index, so a
 * person's own avatars will load exactly like the offered ones.
 *
 * What this browser kept from an earlier choice, while it is still on offer;
 * else a random avatar from the offer, which then becomes the choice. Null
 * when nothing is offered: the engine then wears its own default.
 */
export async function wornAvatar(): Promise<string | null> {
	const set = await offered();
	const before = kept('avatar');
	if (before && set.includes(before)) return before;
	if (set.length === 0) return null;

	const picked = set[crypto.getRandomValues(new Uint32Array(1))[0] % set.length];
	keep('avatar', picked);
	return picked;
}
