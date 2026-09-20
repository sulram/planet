<script lang="ts">
	// One number in a range, with its label and its value in sight. `oninput`
	// fires while dragging: what it tunes should answer at once.
	interface Props {
		label: string;
		value: number;
		min: number;
		max: number;
		step?: number;
		disabled?: boolean;
		/** How the value is shown. Default: two decimals. */
		format?: (value: number) => string;
		/** While dragging: what it tunes should answer at once. */
		oninput?: (value: number) => void;
		/** Once, when the drag ends: for what should not run per frame. */
		onchange?: (value: number) => void;
	}

	let {
		label,
		value,
		min,
		max,
		step = 0.01,
		disabled = false,
		format = (n) => n.toFixed(2),
		oninput,
		onchange
	}: Props = $props();
</script>

<label class="slider" class:slider--off={disabled}>
	<span class="head">
		<span>{label}</span>
		<output>{format(value)}</output>
	</span>
	<input
		type="range"
		{min}
		{max}
		{step}
		{value}
		{disabled}
		oninput={(e) => oninput?.(e.currentTarget.valueAsNumber)}
		onchange={(e) => onchange?.(e.currentTarget.valueAsNumber)}
	/>
</label>

<style>
	.slider {
		display: flex;
		flex-direction: column;
		gap: var(--sp-1);
	}
	.slider--off {
		color: var(--text-muted);
	}
	.head {
		display: flex;
		justify-content: space-between;
		gap: var(--sp-3);
	}
	output {
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
	}
	input {
		appearance: none;
		width: 100%;
		height: var(--sp-4);
		margin: 0;
		background: transparent;
		cursor: pointer;
	}
	input:disabled {
		cursor: not-allowed;
	}
	input::-webkit-slider-runnable-track {
		height: var(--bw);
		background: var(--border-strong);
	}
	input::-moz-range-track {
		height: var(--bw);
		background: var(--border-strong);
	}
	input::-webkit-slider-thumb {
		appearance: none;
		width: var(--sp-3);
		height: var(--sp-4);
		margin-top: calc((var(--bw) - var(--sp-4)) / 2);
		border: none;
		border-radius: var(--radius);
		background: var(--action);
	}
	input::-moz-range-thumb {
		width: var(--sp-3);
		height: var(--sp-4);
		border: none;
		border-radius: var(--radius);
		background: var(--action);
	}
	input:disabled::-webkit-slider-thumb {
		background: var(--text-muted);
	}
	input:disabled::-moz-range-thumb {
		background: var(--text-muted);
	}
	input:focus-visible {
		outline: var(--bw) solid var(--focus);
		outline-offset: var(--sp-1);
	}
</style>
