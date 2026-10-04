<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import { page } from '$app/state';
	import { keepLocale, locales, t, type Locale } from '$lib/i18n';
	import Segmented from './Segmented.svelte';

	const options = locales.map((value) => ({ value, label: value }));
	const current = $derived(page.data.locale);

	// Write the cookie and rerun every `load`: `t()` follows, and what is on
	// the screen survives.
	async function choose(locale: Locale) {
		if (locale === current) return;
		keepLocale(locale);
		await invalidateAll();
	}
</script>

<Segmented {options} value={current} label={t('common.language')} onselect={choose} />
