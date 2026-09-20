<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		label: string;
		/** id of the inner control, for the label's `for`. */
		for: string;
		hint?: string;
		/** Shown whole, in place of the hint. */
		error?: string;
		children: Snippet;
	}

	let { label, for: forId, hint, error, children }: Props = $props();
</script>

<div class="field">
	<label for={forId}>{label}</label>
	{@render children()}
	{#if error}
		<p class="note error" role="alert">{error}</p>
	{:else if hint}
		<p class="note">{hint}</p>
	{/if}
</div>

<style>
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
