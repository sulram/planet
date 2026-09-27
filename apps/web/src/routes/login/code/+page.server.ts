import { fail, redirect } from '@sveltejs/kit';
import { clearOtp, HOME, otpRequestError, readOtp, safePath, writeOtp } from '$lib/server/auth';
import { pbStatus } from '$lib/server/pb';
import { translate } from '$lib/i18n/config';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = ({ cookies, locals }) => {
	if (locals.user) redirect(303, HOME);
	const otp = readOtp(cookies);
	if (!otp) redirect(303, '/login');
	return { email: otp.email };
};

export const actions: Actions = {
	verify: async ({ request, locals, cookies }) => {
		const otp = readOtp(cookies);
		if (!otp) redirect(303, '/login');

		const form = await request.formData();
		const code = String(form.get('code') ?? '').replace(/\s+/g, '');
		if (!code) return fail(400, { error: translate(locals.locale, 'auth.error.codeRequired') });

		try {
			await locals.pb.collection('users').authWithOTP(otp.otpId, code);
		} catch (err) {
			const key = pbStatus(err) === 0 ? 'auth.error.unreachable' : 'auth.error.codeInvalid';
			return fail(400, { error: translate(locals.locale, key) });
		}

		// The session sits in the auth store; hooks.server.ts writes the cookie.
		clearOtp(cookies);
		redirect(303, safePath(otp.redirect) ?? HOME);
	},

	resend: async ({ locals, cookies }) => {
		const otp = readOtp(cookies);
		if (!otp) redirect(303, '/login');

		try {
			const { otpId } = await locals.pb
				.collection('users')
				.requestOTP(otp.email, { body: { email: otp.email, locale: locals.locale } });
			writeOtp(cookies, { ...otp, otpId });
		} catch (err) {
			return fail(502, { error: translate(locals.locale, otpRequestError(pbStatus(err))) });
		}
		return { resent: true };
	}
};
