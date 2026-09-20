import { dev } from '$app/environment';
import type { Cookies } from '@sveltejs/kit';

/** Where a session lands when no destination was asked for. */
export const HOME = '/';

/** Accepts only a same origin path as a destination (no open redirect). */
export function safePath(raw: string | null | undefined): string | null {
	if (!raw || !raw.startsWith('/') || raw.startsWith('//') || raw.includes('\\')) return null;
	return raw;
}

export function loginPath(redirectTo: string): string {
	return `/login?redirect=${encodeURIComponent(redirectTo)}`;
}

/**
 * A pending one-time code request: what `/login/code` and `/login/verify` need
 * to finish the login that `/login` started. Short lived and httpOnly.
 */
export interface OtpState {
	otpId: string;
	email: string;
	redirect: string;
}

const OTP_COOKIE = 'pb_otp';
const OTP_MAX_AGE = 60 * 15;

export function writeOtp(cookies: Cookies, state: OtpState): void {
	cookies.set(OTP_COOKIE, JSON.stringify(state), {
		path: '/login',
		httpOnly: true,
		secure: !dev,
		sameSite: 'lax',
		maxAge: OTP_MAX_AGE
	});
}

export function readOtp(cookies: Cookies): OtpState | null {
	const raw = cookies.get(OTP_COOKIE);
	if (!raw) return null;
	try {
		const v: unknown = JSON.parse(raw);
		if (
			typeof v === 'object' &&
			v !== null &&
			'otpId' in v &&
			typeof v.otpId === 'string' &&
			'email' in v &&
			typeof v.email === 'string' &&
			'redirect' in v &&
			typeof v.redirect === 'string'
		) {
			return { otpId: v.otpId, email: v.email, redirect: v.redirect };
		}
	} catch {
		// malformed cookie: same as no pending request
	}
	return null;
}

export function clearOtp(cookies: Cookies): void {
	cookies.delete(OTP_COOKIE, { path: '/login' });
}

/** The message key for a failed one-time code request, by PocketBase status. */
export function otpRequestError(status: number): 'auth.error.unreachable' | 'auth.error.tooMany' | 'auth.error.sendFailed' {
	if (status === 0) return 'auth.error.unreachable';
	if (status === 429) return 'auth.error.tooMany';
	return 'auth.error.sendFailed';
}
