<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		label: string;
		/** id of the inner control. Left out for a group, which names itself. */
		for?: string;
		hint?: string;
		/** Shown whole, in place of the hint. */
		error?: string;
		children: Snippet;
	}

	let { label, for: forId, hint, error, children }: Props = $props();
</script>

<div class="field">
	{#if forId}
		<label for={forId}>{label}</label>
	{:else}
		<span class="label">{label}</span>
	{/if}
	{@render children()}
	{#if error}
		<p class="note error" role="alert">{error}</p>
	{:else if hint}
		<p class="note">{hint}</p>
	{/if}
</div>

<style>
	label,
	.label {
		display: block;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--sp-2);
	}
	label {
		font-weight: var(--fw-medium);
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
		color: var(--text-muted);
	}
	.note {
		color: var(--text-muted);
	}
	.error {
		color: var(--danger);
	}
	.error::before {
		content: '! ';
		font-weight: var(--fw-bold);
	}
</style>
