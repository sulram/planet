<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		/** narrow: one form. wide: lists and tables. */
		size?: 'narrow' | 'wide';
		children: Snippet;
	}

	let { size = 'wide', children }: Props = $props();
</script>

<main class="page page--{size}">
	{@render children()}
</main>

<style>
	.page {
		margin: 0 auto;
		padding: var(--sp-7) var(--sp-5) var(--sp-8);
	}
	.page--narrow {
		max-width: calc(var(--page-narrow) + 2 * var(--sp-5));
	}
	.page--wide {
		max-width: calc(var(--page-wide) + 2 * var(--sp-5));
	}
</style>
