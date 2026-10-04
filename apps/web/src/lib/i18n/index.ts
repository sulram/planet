/**
 * i18n: the import point for components.
 *
 *   import { t } from '$lib/i18n';
 *   <h1>{t('auth.login.title')}</h1>
 *
 * On the server (actions, load, hooks) use `translate(locals.locale, key)`.
 */
import { page } from '$app/state';
import { fill, formatDate, translate, type Locale, type MessageKey } from './config';

export * from './config';

/** Component `t`: uses the request locale from the root layout. Reactive. */
export function t(key: MessageKey, params?: Record<string, string | number>): string {
	return translate(page.data.locale, key, params);
}

/**
 * A component `t` over a catalogue of its own. A plugin carries its words in
 * its own folder, a catalogue for each locale, and reads them through this:
 *
 *   export const t = words({ en, pt });
 */
export function words<M extends Record<string, string>>(catalogs: Record<Locale, M>) {
	return (key: keyof M, params?: Record<string, string | number>): string => fill(catalogs[page.data.locale][key], params);
}

/** Component date formatter, in the request locale. */
export function d(iso: string): string {
	return formatDate(page.data.locale, iso);
}
