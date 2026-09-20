import { dev } from '$app/environment';
import { redirect } from '@sveltejs/kit';
import { isLocale } from '$lib/i18n/config';
import { HOME, safePath } from '$lib/server/auth';
import type { RequestHandler } from './$types';

// Language switch without JavaScript: store the choice in the `lang` cookie
// and go back. The next request resolves the locale again during SSR.
export const POST: RequestHandler = async ({ request, cookies }) => {
	const form = await request.formData();
	const locale = String(form.get('locale') ?? '');
	if (isLocale(locale)) {
		cookies.set('lang', locale, {
			path: '/',
			httpOnly: false,
			secure: !dev,
			sameSite: 'lax',
			maxAge: 60 * 60 * 24 * 365
		});
	}
	redirect(303, safePath(String(form.get('redirect') ?? '')) ?? HOME);
};
