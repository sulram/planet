<script lang="ts">
	import type { Snippet } from 'svelte';

	// Data table frame: border, horizontal scroll, cell style. The <table> comes
	// from the consumer (children); the style reaches it through :global.
	let { children }: { children: Snippet } = $props();
</script>

<div class="table">
	{@render children()}
</div>

<style>
	.table {
		overflow-x: auto;
		border: var(--bw) solid var(--border-strong);
	}
	.table :global(table) {
		width: 100%;
		border-collapse: collapse;
	}
	.table :global(th),
	.table :global(td) {
		padding: var(--sp-3) var(--sp-4);
		text-align: left;
		vertical-align: middle;
		white-space: nowrap;
	}
	.table :global(thead th) {
		border-bottom: var(--bw) solid var(--border-strong);
		background: var(--bg-inset);
		color: var(--text-muted);
		font-weight: var(--fw-medium);
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
	}
	.table :global(tbody tr + tr td) {
		border-top: var(--bw) solid var(--border);
	}
	.table :global(tbody tr:hover td) {
		background: var(--bg-inset);
	}
	.table :global(td.muted) {
		color: var(--text-muted);
	}
	.table :global(td.actions) {
		text-align: right;
	}
	.table :global(td.actions > *) {
		margin-left: var(--sp-2);
	}
</style>
