<script lang="ts">
	import { t } from '$lib/i18n';
	import type { Line } from './Chat.svelte';
	import type { Anchor, PeerInfo } from './index';
	import { who } from './who';

	// Labels hung over heads: a nametag for everyone else, and what anyone
	// just said in a balloon, yours included. The engine says where each head
	// is on the screen every frame; this only draws there.
	interface Props {
		anchors: Anchor[];
		peers: PeerInfo[];
		/** This client's own session, while online. */
		me: number | null;
		lines: Line[];
	}

	let { anchors, peers, me, lines }: Props = $props();

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

	function name(session: number): string {
		const peer = peers.find((p) => p.session === session);
		return peer ? who(peer) : '';
	}
</script>

<div class="balloons" aria-hidden="true">
	{#each anchors as anchor (anchor.session)}
		{@const line = saying.get(anchor.session)}
		{@const own = anchor.session === me}
		{#if anchor.distance_m < FAR_M && (line || !own)}
			<div class="head" style:left="{anchor.x * 100}%" style:top="{anchor.y * 100}%" style:--near={Math.min(1, 12 / anchor.distance_m)}>
				{#if line}
					{#key line.id}
						<div class="balloon" style:animation-duration="{BALLOON_MS}ms">
							{#if line.text}{line.text}{:else}{t('engine.chat.here')}{/if}
						</div>
					{/key}
				{/if}
				{#if !own}
					<div class="name">{name(anchor.session)}</div>
				{/if}
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
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--sp-1);
		/* Floats a little over the anchor, so it never touches the head, and
		   shrinks with distance, never below half. */
		transform: translate(-50%, calc(-100% - var(--sp-8))) scale(clamp(0.5, var(--near), 1));
		transform-origin: bottom center;
	}
	.name {
		padding: 0 var(--sp-2);
		background: var(--bg-overlay);
		color: var(--text);
		white-space: nowrap;
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
