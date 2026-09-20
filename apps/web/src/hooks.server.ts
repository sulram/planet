import { sequence } from '@sveltejs/kit/hooks';
import { dev } from '$app/environment';
import type { Handle, HandleServerError } from '@sveltejs/kit';
import { createServerClient, type SessionUser } from '$lib/server/pb';
import { HOME, loginPath } from '$lib/server/auth';
import { resolveLocale } from '$lib/i18n/config';
import { resolveTheme, THEME_COOKIE } from '$lib/ds/theme';

const AUTH_COOKIE = 'pb_auth';

// Locale and theme per request, stamped on <html> during SSR: no flash.
// Locale: cookie `lang` (explicit choice), then Accept-Language, then default.
// Theme: cookie `theme` when the person chose one; empty means "follow the OS".
const stamp: Handle = async ({ event, resolve }) => {
	const locale = resolveLocale(event.cookies.get('lang'), event.request.headers.get('accept-language'));
	const theme = resolveTheme(event.cookies.get(THEME_COOKIE)) ?? '';
	event.locals.locale = locale;
	return resolve(event, {
		transformPageChunk: ({ html }) => html.replace('%lang%', locale).replace('%theme%', theme)
	});
};

// Session by httpOnly cookie. The token never reaches storage readable by JS.
// Every request revalidates against PocketBase; the raw cookie is never trusted.
const session: Handle = async ({ event, resolve }) => {
	const pb = createServerClient();
	pb.authStore.loadFromCookie(event.request.headers.get('cookie') ?? '', AUTH_COOKIE);

	try {
		if (pb.authStore.isValid) await pb.collection('users').authRefresh();
	} catch {
		// invalid or expired token, or PocketBase unreachable: signed out
		pb.authStore.clear();
	}

	event.locals.pb = pb;
	const rec = pb.authStore.record;
	event.locals.user = rec
		? ({
				id: rec.id,
				email: String(rec.email ?? ''),
				name: String(rec.name ?? ''),
				operator: rec.operator === true,
				verified: rec.verified === true
			} satisfies SessionUser)
		: null;

	// The backoffice is for operators only, and the barrier lives HERE: a layout
	// guard covers `load` alone, and form actions do not pass through it.
	const { pathname, search } = event.url;
	if (pathname === '/backoffice' || pathname.startsWith('/backoffice/')) {
		const user = event.locals.user;
		if (!user?.operator) {
			if (event.request.method !== 'GET' && event.request.method !== 'HEAD') {
				return new Response('Forbidden', { status: 403 });
			}
			const location = user ? HOME : loginPath(pathname + search);
			return new Response(null, { status: 303, headers: { location } });
		}
	}

	const response = await resolve(event);
	response.headers.append(
		'set-cookie',
		pb.authStore.exportToCookie({ httpOnly: true, secure: !dev, sameSite: 'Lax' }, AUTH_COOKIE)
	);
	return response;
};

export const handle = sequence(stamp, session);

export const handleError: HandleServerError = ({ error, event }) => {
	console.error(`[500 ${event.request.method} ${event.url.pathname}]`, error);
};
