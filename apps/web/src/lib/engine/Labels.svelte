<script lang="ts">
	import type { Label } from './protocol';

	// A landmark's name hung over its star. The engine says where every
	// named part in view is on the screen each frame, within the reach of a
	// nametag; this only draws there, as `Nametags` does over a head.
	interface Props {
		labels: Label[];
	}

	let { labels }: Props = $props();
</script>

<div class="labels" aria-hidden="true">
	{#each labels as label (label.id)}
		<div class="over" style:left="{label.x * 100}%" style:top="{label.y * 100}%" style:--near={Math.min(1, 12 / label.distance_m)}>
			<div class="name">{label.text}</div>
		</div>
	{/each}
</div>

<style>
	.labels {
		position: absolute;
		inset: 0;
		overflow: hidden;
		pointer-events: none;
	}
	.over {
		position: absolute;
		/* Floats a little over the star, and shrinks with distance, never
		   below half. */
		transform: translate(-50%, calc(-100% - var(--sp-4))) scale(clamp(0.5, var(--near), 1));
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
