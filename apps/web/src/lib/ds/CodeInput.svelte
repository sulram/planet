<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';

	// A one-time code, one cell per digit, in two halves the way the email
	// spells it. One real input lies over the cells, transparent: typing,
	// pasting "3635 7477", the phone's numeric keypad and the browser's
	// autofill from the mail all reach it, and the cells only draw what it
	// holds. What is not a digit is dropped as it arrives, so a paste with
	// the space fits. The ring sits on the cell the next digit lands in.
	interface Props extends Omit<HTMLInputAttributes, 'value' | 'type' | 'maxlength' | 'oninput'> {
		value?: string;
		/** Digits in a code: PocketBase's default. */
		length?: number;
		/** Marks the code as refused; pair with `Field error`. */
		invalid?: boolean;
	}

	let { value = $bindable(''), length = 8, invalid = false, ...rest }: Props = $props();

	const cells = $derived(Array.from({ length }, (_, i) => value[i] ?? ''));
	const half = $derived(length % 2 === 0 ? length / 2 : -1);
	/** The cell the next digit lands in; the last one once the code is whole. */
	const next = $derived(Math.min(value.length, length - 1));

	function take(event: Event & { currentTarget: HTMLInputElement }) {
		const digits = event.currentTarget.value.replace(/\D/g, '').slice(0, length);
		event.currentTarget.value = digits;
		value = digits;
	}
</script>

<div class="code" class:code--invalid={invalid}>
	<input
		{...rest}
		{value}
		type="text"
		inputmode="numeric"
		autocomplete="one-time-code"
		autocapitalize="off"
		spellcheck="false"
		aria-invalid={invalid || undefined}
		oninput={take}
	/>
	<div class="cells" aria-hidden="true">
		{#each cells as digit, i (i)}
			<span class="cell" class:cell--filled={digit} class:cell--next={i === next} class:cell--half={i === half}>{digit}</span>
		{/each}
	</div>
</div>

<style>
	.code {
		position: relative;
		display: flex;
		/* keeps its own width inside a stretching column */
		align-self: flex-start;
	}
	/* over the cells, whole, so a click anywhere lands in it; seen through */
	input {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		padding: 0;
		border: none;
		background: transparent;
		color: transparent;
		caret-color: transparent;
		outline: none;
		cursor: text;
	}
	input::selection {
		background: transparent;
	}
	.cells {
		display: flex;
		gap: var(--sp-2);
	}
	.cell {
		display: grid;
		place-items: center;
		width: var(--sp-7);
		height: var(--sp-7);
		border: var(--bw) solid var(--border);
		font-weight: var(--fw-bold);
		font-variant-numeric: tabular-nums;
	}
	/* the break between the halves, as in the email */
	.cell--half {
		margin-left: var(--sp-3);
	}
	.cell--filled {
		border-color: var(--border-strong);
	}
	.code:focus-within .cell--next {
		outline: calc(2 * var(--bw)) solid var(--focus);
		outline-offset: var(--sp-1);
	}
	.code--invalid .cell {
		border-color: var(--danger);
	}
	.code:has(input:disabled) .cells {
		opacity: 0.4;
	}
	.code:has(input:disabled) input {
		cursor: not-allowed;
	}
</style>
