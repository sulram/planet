import { newLanguage } from '$lib/door';
import { htmlLang, isLocale, keepLocale, resolveLocale } from '$lib/i18n/config';
import type { LayoutLoad } from './$types';

// The page is files (DECISIONS 90): every route is built once as a shell the
// browser fills.
export const ssr = false;
export const prerender = true;

/**
 * The language: the newest choice, made in mundos and said by its door or
 * made here with the switch, kept in the `lang` cookie; then the browser's
 * own list, then the default. Read again whenever the choice changes.
 */
export const load: LayoutLoad = () => {
	const said = newLanguage();
	if (said && isLocale(said)) keepLocale(said);
	const kept = /(?:^|; )lang=([^;]+)/.exec(document.cookie)?.[1];
	const locale = resolveLocale(kept, navigator.languages.join(','));
	document.documentElement.lang = htmlLang[locale];
	return { locale };
};
