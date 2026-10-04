/**
 * i18n core, free of component runtime: safe on the server and in shared code.
 * The component `t` lives in `./index.ts`.
 */
import { en, type MessageKey, type Messages } from './en';
import { pt } from './pt';
import { zh } from './zh';

/** The languages the page speaks: those mundos speaks, by the same codes. */
export const locales = ['en', 'pt', 'zh'] as const;
export type Locale = (typeof locales)[number];
export const defaultLocale: Locale = 'en';

const catalogs: Record<Locale, Messages> = { en, pt, zh };

/** What `<html lang>` says for each: the script and region a font is picked by. */
export const htmlLang: Record<Locale, string> = { en: 'en', pt: 'pt-BR', zh: 'zh-CN' };

export function isLocale(value: string): value is Locale {
	return (locales as readonly string[]).includes(value);
}

/** Keeps a language as this browser's choice, and has the page say it. In the browser only. */
export function keepLocale(locale: Locale): void {
	document.cookie = `lang=${locale}; path=/; max-age=${60 * 60 * 24 * 365}; samesite=lax`;
	document.documentElement.lang = htmlLang[locale];
}

/** Request locale: explicit cookie, then Accept-Language, then the default. */
export function resolveLocale(cookie?: string, acceptLanguage?: string | null): Locale {
	if (cookie && isLocale(cookie)) return cookie;
	if (acceptLanguage) {
		for (const part of acceptLanguage.split(',')) {
			const base = part.split(';')[0].trim().toLowerCase().split('-')[0];
			if (isLocale(base)) return base;
		}
	}
	return defaultLocale;
}

/** Fills a message in. Interpolation: `{name}` in the text, `{ name }` in params. */
export function fill(message: string, params?: Record<string, string | number>): string {
	let msg = message;
	if (params) {
		for (const k in params) msg = msg.replaceAll(`{${k}}`, String(params[k]));
	}
	return msg;
}

/** Translates a key. */
export function translate(locale: Locale, key: MessageKey, params?: Record<string, string | number>): string {
	return fill(catalogs[locale][key], params);
}

/** Day of a PocketBase timestamp, in the reader's locale. UTC, so SSR and browser agree. */
export function formatDate(locale: Locale, iso: string): string {
	const date = new Date(iso);
	if (!iso || Number.isNaN(date.getTime())) return '';
	return new Intl.DateTimeFormat(locale, { dateStyle: 'medium', timeZone: 'UTC' }).format(date);
}

export type { Messages, MessageKey };
