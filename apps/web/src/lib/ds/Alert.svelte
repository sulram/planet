<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		variant?: 'info' | 'danger';
		title?: string;
		children: Snippet;
	}

	let { variant = 'info', title, children }: Props = $props();
</script>

<div class="alert alert--{variant}" role={variant === 'danger' ? 'alert' : 'status'}>
	<span class="mark" aria-hidden="true">{variant === 'danger' ? '!' : 'i'}</span>
	<div class="body">
		{#if title}<strong class="title">{title}</strong>{/if}
		<div>{@render children()}</div>
	</div>
</div>

<style>
	.alert {
		display: flex;
		gap: var(--sp-3);
		padding: var(--sp-3) var(--sp-4);
		border: var(--bw) solid var(--border-strong);
		background: var(--bg);
		color: var(--text);
		max-width: var(--measure);
	}
	.alert--danger {
		border-color: var(--danger);
		color: var(--danger);
	}
	.mark {
		flex: none;
		font-weight: var(--fw-bold);
	}
	.body {
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.title {
		display: block;
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
	}
</style>
