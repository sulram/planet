<script lang="ts">
	import { invalidateAll } from '$app/navigation';
	import { page } from '$app/state';
	import { keepLocale, languageNames, locales, t, type Locale } from '$lib/i18n';
	import Segmented from './Segmented.svelte';

	// The language, chosen by its code in a row where room is short (a top
	// bar), or by its name in a list, one a line, where a panel opens for it.
	interface Props {
		layout?: 'row' | 'list';
		/** Hears every pick, the current language included: a panel closes on it. */
		onchoose?: (locale: Locale) => void;
	}

	let { layout = 'row', onchoose }: Props = $props();

	const options = locales.map((value) => ({ value, label: value }));
	const current = $derived(page.data.locale);

	// Write the cookie and rerun every `load`: `t()` follows, and what is on
	// the screen survives.
	async function choose(locale: Locale) {
		onchoose?.(locale);
		if (locale === current) return;
		keepLocale(locale);
		await invalidateAll();
	}
</script>

{#if layout === 'list'}
	<ul class="list" aria-label={t('common.language')}>
		{#each locales as locale (locale)}
			<li>
				<button type="button" lang={locale} aria-pressed={locale === current} onclick={() => choose(locale)}>{languageNames[locale]}</button>
			</li>
		{/each}
	</ul>
{:else}
	<Segmented {options} value={current} label={t('common.language')} onselect={choose} />
{/if}

<style>
	.list {
		display: flex;
		flex-direction: column;
		gap: var(--sp-1);
		list-style: none;
	}
	button {
		display: block;
		width: 100%;
		min-height: var(--control-h);
		padding: 0 var(--sp-3);
		border: var(--bw) solid transparent;
		background: transparent;
		color: var(--text-muted);
		text-align: left;
		cursor: pointer;
	}
	button:hover {
		border-color: var(--border-strong);
		color: var(--text);
	}
	button[aria-pressed='true'] {
		background: var(--action);
		color: var(--action-text);
		cursor: default;
	}
</style>
