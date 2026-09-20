import { redirect } from '@sveltejs/kit';
import { clearOtp, HOME, readOtp, safePath } from '$lib/server/auth';
import type { PageServerLoad } from './$types';

// Consumes the one click link of the email: otpId + code arrive in the URL.
// Success: session (hooks.server.ts writes the cookie) and redirect. When the
// link opens in the browser that asked for it, the pending request still holds
// the destination. Failure: the page renders its error state.
export const load: PageServerLoad = async ({ url, locals, cookies }) => {
	const otpId = url.searchParams.get('otpId');
	const code = url.searchParams.get('code');
	if (!otpId || !code) return {};

	try {
		await locals.pb.collection('users').authWithOTP(otpId, code);
	} catch {
		return {};
	}

	const otp = readOtp(cookies);
	clearOtp(cookies);
	redirect(303, (otp?.otpId === otpId ? safePath(otp.redirect) : null) ?? HOME);
};
