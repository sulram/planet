<script lang="ts">
	import type { Anchor, PeerInfo } from './index';
	import { who } from './who';

	// A nametag hung over every head but your own. The engine says where each
	// head is on the screen every frame; this only draws there. What a plugin
	// hangs over a head, a balloon, it hangs above this.
	interface Props {
		anchors: Anchor[];
		peers: PeerInfo[];
		/** This client's own session, while online. */
		me: number | null;
	}

	let { anchors, peers, me }: Props = $props();

	/** Beyond this a head is a dot and a label would be noise. */
	const FAR_M = 120;

	function name(session: number): string {
		const peer = peers.find((p) => p.session === session);
		return peer ? who(peer) : '';
	}
</script>

<div class="nametags" aria-hidden="true">
	{#each anchors as anchor (anchor.session)}
		{#if anchor.distance_m < FAR_M && anchor.session !== me}
			<div class="head" style:left="{anchor.x * 100}%" style:top="{anchor.y * 100}%" style:--near={Math.min(1, 12 / anchor.distance_m)}>
				<div class="name">{name(anchor.session)}</div>
			</div>
		{/if}
	{/each}
</div>

<style>
	.nametags {
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
	.name {
		color: var(--on-world);
		font-weight: var(--fw-bold);
		text-shadow:
			0 0 var(--sp-1) var(--on-world-shadow),
			0 0 var(--sp-1) var(--on-world-shadow),
			0 var(--bw) var(--sp-1) var(--on-world-shadow);
		white-space: nowrap;
	}
</style>
