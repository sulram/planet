import type { Cookies, RequestEvent } from '@sveltejs/kit';
import { dev } from '$app/environment';

const COOKIE = 'name';
/** The most a name carries, as the world keeps it (`NameChars` in the actor). */
export const NAME_CHARS = 24;

/** A name as the world keeps it: one line, trimmed, cut at the limit. Empty is allowed. */
export function cleanName(raw: string): string {
	return [...raw.split(/\s+/).filter(Boolean).join(' ')].slice(0, NAME_CHARS).join('').trim();
}

function keep(cookies: Cookies, name: string) {
	cookies.set(COOKIE, name, {
		path: '/',
		httpOnly: true,
		secure: !dev,
		sameSite: 'lax',
		maxAge: 60 * 60 * 24 * 365
	});
}

/**
 * What this person is called in a world: the account's name for someone
 * signed in, else what the visitor chose before, kept in a cookie. Empty
 * when nobody said.
 */
export function visitorName({ locals, cookies }: RequestEvent): string {
	if (locals.user) return cleanName(locals.user.name);
	return cleanName(cookies.get(COOKIE) ?? '');
}

/**
 * Records a name chosen in the world: on the account when signed in, so the
 * next ticket carries it, else in the visitor's cookie.
 */
export async function chooseName({ locals, cookies }: RequestEvent, raw: string): Promise<string> {
	const name = cleanName(raw);
	if (locals.user) await locals.pb.collection('users').update(locals.user.id, { name });
	else keep(cookies, name);
	return name;
}
