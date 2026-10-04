import type { Locale } from '$lib/i18n/config';

declare global {
	namespace App {
		interface PageData {
			locale: Locale;
		}
	}
}

export {};
