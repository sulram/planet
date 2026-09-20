/**
 * Reactive theme state for the browser. The source of truth is
 * `<html data-theme>`, stamped by the server from the `theme` cookie.
 */
import { browser } from '$app/environment';
import { resolveTheme, THEME_COOKIE, type Theme } from './theme';

const YEAR = 60 * 60 * 24 * 365;

function current(): Theme {
	if (!browser) return 'light';
	return (
		resolveTheme(document.documentElement.dataset.theme) ??
		(matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
	);
}

class ThemeState {
	value = $state<Theme>(current());

	set(theme: Theme) {
		this.value = theme;
		if (!browser) return;
		document.documentElement.dataset.theme = theme;
		document.cookie = `${THEME_COOKIE}=${theme}; path=/; max-age=${YEAR}; samesite=lax`;
	}

	toggle() {
		this.set(this.value === 'dark' ? 'light' : 'dark');
	}
}

export const theme = new ThemeState();
