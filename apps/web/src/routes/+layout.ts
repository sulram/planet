import { resolveLocale } from '$lib/i18n/config';
import type { LayoutLoad } from './$types';

// The page is files (DECISIONS 90): every route is built once as a shell the
// browser fills.
export const ssr = false;
export const prerender = true;

/**
 * The language: the choice kept in the `lang` cookie, then the browser's own
 * list, then the default. Read again whenever the choice changes.
 */
export const load: LayoutLoad = () => {
	const kept = /(?:^|; )lang=([^;]+)/.exec(document.cookie)?.[1];
	const locale = resolveLocale(kept, navigator.languages.join(','));
	document.documentElement.lang = locale;
	return { locale };
};
