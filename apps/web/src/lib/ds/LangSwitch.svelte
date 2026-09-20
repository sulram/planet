<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import { page } from '$app/state';
	import { locales, t, type Locale } from '$lib/i18n';
	import Segmented from './Segmented.svelte';

	const options = locales.map((value) => ({ value, label: value }));
	const current = $derived(page.data.locale);
	const redirectTo = $derived(page.url.pathname + page.url.search);

	// With JavaScript: write the cookie and rerun every `load`; `t()` follows,
	// screen and form state survive. Without: the form posts to /locale.
	async function choose(locale: Locale, event: MouseEvent) {
		event.preventDefault();
		if (locale === current) return;
		document.cookie = `lang=${locale}; path=/; max-age=${60 * 60 * 24 * 365}; samesite=lax`;
		document.documentElement.lang = locale;
		await invalidateAll();
	}
</script>

<form method="POST" action="/locale">
	<input type="hidden" name="redirect" value={redirectTo} />
	<Segmented {options} value={current} name="locale" label={t('common.language')} onselect={choose} />
</form>

<style>
	form {
		display: inline-flex;
	}
</style>
