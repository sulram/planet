<script lang="ts">
	import type { Snippet } from 'svelte';

	// A bordered box that floats over the engine canvas. It positions itself in
	// a corner of the nearest positioned ancestor.
	interface Props {
		title: string;
		/** Sits opposite the title, for example a way out. */
		aside?: Snippet;
		corner?: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';
		children: Snippet;
	}

	let { title, aside, corner = 'top-left', children }: Props = $props();
</script>

<section class="panel panel--{corner}">
	<header>
		<h2>{title}</h2>
		{#if aside}{@render aside()}{/if}
	</header>
	{@render children()}
</section>

<style>
	.panel {
		position: absolute;
		z-index: var(--z-panel);
		display: flex;
		flex-direction: column;
		gap: var(--sp-4);
		width: min(var(--panel-w), calc(100% - 2 * var(--sp-4)));
		max-height: calc(100% - 2 * var(--sp-4));
		overflow-y: auto;
		padding: var(--sp-4);
		border: var(--bw) solid var(--border-strong);
		background: var(--bg-overlay);
		color: var(--text);
		backdrop-filter: blur(var(--sp-2));
	}
	.panel--top-left {
		top: var(--sp-4);
		left: var(--sp-4);
	}
	.panel--top-right {
		top: var(--sp-4);
		right: var(--sp-4);
	}
	/* A panel from a bottom corner stops short of the row of buttons in the
	   top corners, so a tall one never covers the button above it. */
	.panel--bottom-left,
	.panel--bottom-right {
		max-height: calc(100% - 3 * var(--sp-4) - var(--control-h) - 2 * var(--bw));
	}
	.panel--bottom-left {
		bottom: var(--sp-4);
		left: var(--sp-4);
	}
	.panel--bottom-right {
		bottom: var(--sp-4);
		right: var(--sp-4);
	}
	header {
		display: flex;
		justify-content: space-between;
		gap: var(--sp-3);
	}
	h2 {
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
	}
</style>
