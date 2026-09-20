import type PocketBase from 'pocketbase';
import type { SessionUser } from '$lib/server/pb';
import type { Locale } from '$lib/i18n/config';

declare global {
	namespace App {
		interface Locals {
			/** PocketBase client of this request, session already loaded from the cookie. */
			pb: PocketBase;
			/** Authenticated (revalidated) user, or null. */
			user: SessionUser | null;
			locale: Locale;
		}
		interface PageData {
			locale: Locale;
			user: SessionUser | null;
		}
	}
}

export {};
