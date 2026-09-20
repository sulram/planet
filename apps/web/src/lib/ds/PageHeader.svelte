<script lang="ts">
	import type { Snippet } from 'svelte';
	import { t } from '$lib/i18n';

	// Page heading: trail, title, lede and actions, always the same.
	// Every page opens with it; never hand build an h1.
	export interface Crumb {
		label: string;
		href: string;
	}
	interface Props {
		title: string;
		lede?: string;
		trail?: Crumb[];
		actions?: Snippet;
	}

	let { title, lede, trail = [], actions }: Props = $props();
</script>

<header class="ph">
	{#if trail.length}
		<nav class="trail" aria-label={t('common.trail')}>
			{#each trail as crumb (crumb.href)}
				<a href={crumb.href}>{crumb.label}</a>
				<span aria-hidden="true">/</span>
			{/each}
		</nav>
	{/if}
	<div class="row">
		<div class="titles">
			<h1>{title}</h1>
			{#if lede}<p class="lede">{lede}</p>{/if}
		</div>
		{#if actions}<div class="actions">{@render actions()}</div>{/if}
	</div>
</header>

<style>
	.ph {
		margin-bottom: var(--sp-6);
		padding-bottom: var(--sp-4);
		border-bottom: var(--bw) solid var(--border-strong);
	}
	.trail {
		display: flex;
		gap: var(--sp-2);
		margin-bottom: var(--sp-2);
		color: var(--text-muted);
	}
	.trail a {
		text-decoration: none;
	}
	.trail a:hover {
		color: var(--text);
	}
	.row {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		flex-wrap: wrap;
		gap: var(--sp-4);
	}
	.titles {
		min-width: 0;
	}
	h1 {
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
	}
	.lede {
		max-width: var(--measure);
		margin-top: var(--sp-2);
		color: var(--text-muted);
	}
	.actions {
		display: flex;
		gap: var(--sp-3);
	}
</style>
