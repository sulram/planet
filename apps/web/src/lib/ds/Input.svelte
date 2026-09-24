<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';

	interface Props extends HTMLInputAttributes {
		value?: string;
		/** Marks the control as failing validation; pair with `Field error`. */
		invalid?: boolean;
		/** The element itself, for a caller that moves focus. */
		element?: HTMLInputElement;
	}

	let { value = $bindable(''), invalid = false, element = $bindable(), ...rest }: Props = $props();
</script>

<input class="input" bind:this={element} bind:value aria-invalid={invalid || undefined} {...rest} />

<style>
	.input {
		display: block;
		width: 100%;
		min-height: var(--control-h);
		padding: 0 var(--sp-3);
		border: var(--bw) solid var(--border-strong);
		border-radius: var(--radius);
		background: var(--bg);
		color: var(--text);
	}
	.input::placeholder {
		color: var(--text-muted);
	}
	.input:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.input:read-only {
		background: var(--bg-inset);
		border-color: var(--border);
		color: var(--text-muted);
	}
	.input[aria-invalid='true'] {
		border-color: var(--danger);
	}
</style>
