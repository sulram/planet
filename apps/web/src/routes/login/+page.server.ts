import { fail, redirect } from '@sveltejs/kit';
import { HOME, otpRequestError, safePath, writeOtp } from '$lib/server/auth';
import { pbStatus } from '$lib/server/pb';
import { translate } from '$lib/i18n/config';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = ({ locals, url }) => {
	const redirectTo = safePath(url.searchParams.get('redirect'));
	if (locals.user) redirect(303, redirectTo ?? HOME);
	return { redirect: redirectTo ?? '' };
};

export const actions: Actions = {
	// Magic link only. The email carries a one-time code AND a one click link
	// (see /login/verify). No account yet? The server creates it on this request.
	default: async ({ request, locals, cookies }) => {
		const form = await request.formData();
		const email = String(form.get('email') ?? '')
			.trim()
			.toLowerCase();
		if (!email) return fail(400, { email, error: translate(locals.locale, 'auth.error.emailRequired') });

		// The reader's language rides with the request: the server keeps it on
		// the account and writes the email in it. The SDK replaces the body
		// it is given, so the email goes along.
		let otpId: string;
		try {
			({ otpId } = await locals.pb.collection('users').requestOTP(email, { body: { email, locale: locals.locale } }));
		} catch (err) {
			const status = pbStatus(err);
			if (status === 400) return fail(400, { email, error: translate(locals.locale, 'auth.error.emailInvalid') });
			return fail(502, { email, error: translate(locals.locale, otpRequestError(status)) });
		}

		writeOtp(cookies, { otpId, email, redirect: safePath(String(form.get('redirect') ?? '')) ?? '' });
		redirect(303, '/login/code');
	}
};
