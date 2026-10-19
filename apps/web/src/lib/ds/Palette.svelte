<script lang="ts">
	// The paints of a world, one swatch each, as they will be laid: the colour
	// in the finish and with the edge picked. The one in hand is pressed.
	interface Props {
		/** The paints, as `#rrggbb`, in order. */
		colors: string[];
		/** Index of the colour in hand. */
		paint: number;
		finish: 'matte' | 'glass' | 'light';
		edge: 'none' | 'black' | 'white';
		/** Accessible name of the group. */
		label: string;
		/** The name of the nth swatch, counted from 1. */
		pick: (n: number) => string;
		onpick: (index: number) => void;
	}

	let { colors, paint, finish, edge, label, pick, onpick }: Props = $props();
</script>

<div class="palette {finish} edge-{edge}" role="group" aria-label={label}>
	{#each colors as color, index (index)}
		<button type="button" class="swatch" style:--paint={color} aria-pressed={index === paint} aria-label={pick(index + 1)} onclick={() => onpick(index)}></button>
	{/each}
</div>

<style>
	.palette {
		display: grid;
		grid-template-columns: repeat(8, 1fr);
		gap: var(--sp-2);
	}
	.swatch {
		aspect-ratio: 1;
		border: none;
		background: var(--paint);
		cursor: pointer;
	}
	.glass .swatch {
		background: linear-gradient(135deg, color-mix(in srgb, var(--paint) 30%, transparent) 50%, color-mix(in srgb, var(--paint) 60%, transparent) 50%);
	}
	.light .swatch {
		box-shadow: 0 0 var(--sp-3) var(--paint);
	}
	.edge-black .swatch {
		box-shadow: inset 0 0 0 calc(2 * var(--bw)) black;
	}
	.edge-white .swatch {
		box-shadow: inset 0 0 0 calc(2 * var(--bw)) white;
	}
	.light.edge-black .swatch {
		box-shadow:
			inset 0 0 0 calc(2 * var(--bw)) black,
			0 0 var(--sp-3) var(--paint);
	}
	.light.edge-white .swatch {
		box-shadow:
			inset 0 0 0 calc(2 * var(--bw)) white,
			0 0 var(--sp-3) var(--paint);
	}
	.swatch[aria-pressed='true'] {
		outline: calc(2 * var(--bw)) solid var(--text);
		outline-offset: var(--bw);
	}
</style>
