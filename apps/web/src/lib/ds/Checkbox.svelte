<script lang="ts">
	import type { Snippet } from 'svelte';

	// A yes or no with its label. The mark is a filled square: the design
	// system has one shape and no icons.
	interface Props {
		checked: boolean;
		disabled?: boolean;
		onchange?: (checked: boolean) => void;
		/** The label: clickable. */
		children: Snippet;
	}

	let { checked, disabled = false, onchange, children }: Props = $props();
</script>

<label class="check" class:check--off={disabled}>
	<input type="checkbox" {checked} {disabled} onchange={(e) => onchange?.(e.currentTarget.checked)} />
	<span class="box" aria-hidden="true"></span>
	<span>{@render children()}</span>
</label>

<style>
	.check {
		display: flex;
		align-items: center;
		gap: var(--sp-3);
		min-height: var(--control-h);
		cursor: pointer;
	}
	.check--off {
		color: var(--text-muted);
		cursor: not-allowed;
	}
	input {
		position: absolute;
		opacity: 0;
		pointer-events: none;
	}
	.box {
		flex-shrink: 0;
		width: var(--sp-4);
		height: var(--sp-4);
		border: var(--bw) solid var(--border-strong);
		background: transparent;
		box-shadow: inset 0 0 0 var(--sp-1) var(--bg);
	}
	.check:has(input:checked) .box {
		background: var(--action);
	}
	.check:has(input:focus-visible) .box {
		outline: var(--bw) solid var(--focus);
		outline-offset: var(--sp-1);
	}
</style>
