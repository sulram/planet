<script lang="ts">
	import { t } from '$lib/i18n';
	import type { Anchor } from '$lib/engine';
	import type { Line } from './protocol';

	// What anyone just said, in a balloon over their head, yours included.
	// The engine says where each head is on the screen every frame; this only
	// draws there, above the nametag the core hangs on everyone else.
	interface Props {
		anchors: Anchor[];
		/** This client's own session, while online. */
		me: number | null;
		lines: Line[];
	}

	let { anchors, me, lines }: Props = $props();

	/** How long a balloon stays up. The fade is the last fifth of it. */
	const BALLOON_MS = 8000;
	/** Beyond this a head is a dot and a label would be noise. */
	const FAR_M = 120;

	let now = $state(Date.now());
	$effect(() => {
		if (lines.length === 0) return;
		const tick = setInterval(() => (now = Date.now()), 500);
		return () => clearInterval(tick);
	});

	/** The newest line each head said, while it is worth a balloon. */
	const saying = $derived.by(() => {
		const latest = new Map<number, Line>();
		for (const line of lines) if (now - line.at < BALLOON_MS) latest.set(line.session, line);
		return latest;
	});
</script>

<div class="balloons" aria-hidden="true">
	{#each anchors as anchor (anchor.session)}
		{@const line = saying.get(anchor.session)}
		{#if line && anchor.distance_m < FAR_M}
			<div
				class="head"
				class:named={anchor.session !== me}
				style:left="{anchor.x * 100}%"
				style:top="{anchor.y * 100}%"
				style:--near={Math.min(1, 12 / anchor.distance_m)}
			>
				{#key line.id}
					<div class="balloon" style:animation-duration="{BALLOON_MS}ms">
						{#if line.text}{line.text}{:else}{t('engine.chat.here')}{/if}
					</div>
				{/key}
			</div>
		{/if}
	{/each}
</div>

<style>
	.balloons {
		position: absolute;
		inset: 0;
		overflow: hidden;
		pointer-events: none;
	}
	.head {
		position: absolute;
		/* Floats a little over the anchor, so it never touches the head, and
		   shrinks with distance, never below half. */
		transform: translate(-50%, calc(-100% - var(--sp-6))) scale(clamp(0.5, var(--near), 1));
		transform-origin: bottom center;
	}
	/* Over a head that wears a nametag, the balloon clears the name's line. */
	.head.named {
		padding-bottom: calc(1lh + var(--sp-3));
	}
	.balloon {
		position: relative;
		max-width: 220px;
		padding: var(--sp-2) var(--sp-3);
		border: var(--bw) solid var(--border-strong);
		background: var(--bg);
		color: var(--text);
		overflow-wrap: anywhere;
		animation: balloon linear forwards;
	}
	/* The tail: a square turned on its point, hanging off the bottom edge. */
	.balloon::after {
		content: '';
		position: absolute;
		left: 50%;
		bottom: calc(-1 * var(--sp-2));
		width: var(--sp-3);
		height: var(--sp-3);
		border: var(--bw) solid var(--border-strong);
		border-top: none;
		border-left: none;
		background: var(--bg);
		transform: translateX(-50%) rotate(45deg);
	}
	@keyframes balloon {
		0% {
			opacity: 0;
			transform: translateY(var(--sp-2));
		}
		3%,
		80% {
			opacity: 1;
			transform: none;
		}
		100% {
			opacity: 0;
		}
	}
</style>
