<script lang="ts">
	import type { Snippet } from 'svelte';
	import Spinner from './Spinner.svelte';

	// What covers the picture until it is the world asked for: a word in the
	// engine's voice, with a spinner while something is on its way, with a
	// sentence and an action when something stands in the world's place. The
	// page shows it before the engine exists, so the first screen and the
	// engine's own waits are one thing.
	interface Props {
		/** The word, in capitals. */
		word?: string;
		/** Something is on its way: the spinner turns before the word. */
		busy?: boolean;
		/** A sentence under the word, in its own case. */
		lede?: string;
		/** Softened out of the way: the world is on the screen. */
		lifted?: boolean;
		/** What stands under the word: an action, or an alert in the word's place. */
		children?: Snippet;
	}

	let { word, busy = false, lede, lifted = false, children }: Props = $props();
</script>

<div class="veil" class:lifted>
	{#if word}
		<div class="wait" role="status">
			{#if busy}<Spinner label={word} />{/if}
			<span class="word">{word}</span>
			{#if lede}<p class="lede">{lede}</p>{/if}
			{#if children}
				<div class="action">{@render children()}</div>
			{/if}
		</div>
	{:else if children}
		{@render children()}
	{/if}
</div>

<style>
	/* Over the picture until it is the world asked for: softened, not hidden,
	   so the wait reads as a world coming and not as a blank. */
	.veil {
		position: absolute;
		inset: 0;
		display: grid;
		place-items: center;
		padding: var(--sp-5);
		pointer-events: none;
		background: color-mix(in srgb, var(--bg) 45%, transparent);
		backdrop-filter: blur(18px);
		-webkit-backdrop-filter: blur(18px);
		transition: opacity 300ms ease;
	}
	.veil.lifted {
		opacity: 0;
		visibility: hidden;
		transition:
			opacity 300ms ease,
			visibility 0s linear 300ms;
	}
	.wait {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--sp-3);
		max-width: var(--measure);
		color: var(--text-muted);
		text-align: center;
	}
	.word {
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
	}
	.lede {
		margin: 0;
	}
	.action {
		margin-top: var(--sp-2);
		pointer-events: auto;
	}
</style>
