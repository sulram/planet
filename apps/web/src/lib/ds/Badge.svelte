<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		variant?: 'outline' | 'solid';
		children: Snippet;
	}

	let { variant = 'outline', children }: Props = $props();
</script>

<span class="badge badge--{variant}">{@render children()}</span>

<style>
	.badge {
		display: inline-block;
		padding: 0 var(--sp-2);
		border: var(--bw) solid var(--border-strong);
		font-weight: var(--fw-medium);
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
		white-space: nowrap;
	}
	.badge--outline {
		color: var(--text);
	}
	.badge--solid {
		background: var(--action);
		color: var(--action-text);
	}
</style>
