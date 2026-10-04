<script lang="ts" generics="T extends string">
	// A row of exclusive options. Inside a form, `name` turns each option into a
	// submit button, so the choice also works without JavaScript.
	interface Option {
		value: T;
		label: string;
		/** The key that picks it, shown small before the label. */
		hint?: string;
	}
	interface Props {
		options: readonly Option[];
		value: T;
		/** Accessible name of the group. */
		label: string;
		name?: string;
		onselect?: (value: T, event: MouseEvent) => void;
	}

	let { options, value, label, name, onselect }: Props = $props();
</script>

<div class="segmented" role="group" aria-label={label}>
	{#each options as opt (opt.value)}
		<button
			type={name ? 'submit' : 'button'}
			{name}
			value={opt.value}
			aria-pressed={opt.value === value}
			onclick={(e) => onselect?.(opt.value, e)}
		>
			{#if opt.hint}<kbd>{opt.hint}</kbd>{/if}{opt.label}
		</button>
	{/each}
</div>

<style>
	.segmented {
		display: inline-flex;
		/* keeps its own width inside a stretching column */
		align-self: flex-start;
		border: var(--bw) solid var(--border-strong);
	}
	button {
		min-height: calc(var(--control-h) - 2 * var(--bw));
		padding: 0 var(--sp-3);
		border: none;
		background: transparent;
		color: var(--text-muted);
		font-weight: var(--fw-medium);
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
		cursor: pointer;
	}
	button:hover {
		color: var(--text);
	}
	button[aria-pressed='true'] {
		background: var(--action);
		color: var(--action-text);
		cursor: default;
	}
	kbd {
		margin-right: var(--sp-2);
		opacity: 0.55;
	}
</style>
