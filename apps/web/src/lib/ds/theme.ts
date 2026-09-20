/**
 * Theme core, safe on the server. The choice lives in the `theme` cookie so SSR
 * can stamp `<html data-theme>` before the first paint. No cookie means no
 * choice: the page follows `prefers-color-scheme`.
 */
export type Theme = 'light' | 'dark';

export const THEME_COOKIE = 'theme';

export function resolveTheme(cookie: string | undefined): Theme | null {
	return cookie === 'light' || cookie === 'dark' ? cookie : null;
}
