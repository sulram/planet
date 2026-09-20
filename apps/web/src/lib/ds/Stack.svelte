<script lang="ts">
	import type { Snippet } from 'svelte';

	// Vertical rhythm for a form or a group of blocks: one gap, no margins.
	let { children }: { children: Snippet } = $props();
</script>

<div class="stack">
	{@render children()}
</div>

<style>
	.stack {
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: var(--sp-4);
	}
</style>
