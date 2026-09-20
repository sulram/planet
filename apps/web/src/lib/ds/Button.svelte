<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';
	import Spinner from './Spinner.svelte';

	// One look for every action. With `href` it renders a link, so navigation
	// stays a real <a> and never nests a button inside one.
	type Variant = 'primary' | 'ghost' | 'danger';

	interface Common {
		variant?: Variant;
		children: Snippet;
	}
	type AsButton = Common &
		Omit<HTMLButtonAttributes, 'children'> & {
			href?: undefined;
			/** Waiting for the server: shows the spinner, blocks the click, announces the wait. */
			loading?: boolean;
		};
	type AsLink = Common &
		Omit<HTMLAnchorAttributes, 'children'> & {
			href: string;
			loading?: undefined;
		};
	type Props = AsButton | AsLink;

	let { variant = 'primary', loading = false, children, ...rest }: Props = $props();
</script>

{#if rest.href !== undefined}
	<a class="btn btn--{variant}" {...rest as HTMLAnchorAttributes}>
		{@render children()}
	</a>
{:else}
	{@const attrs = rest as HTMLButtonAttributes}
	<button
		class="btn btn--{variant}"
		class:btn--loading={loading}
		{...attrs}
		disabled={attrs.disabled || loading}
		aria-busy={loading || undefined}
	>
		{#if loading}<Spinner />{/if}
		{@render children()}
	</button>
{/if}

<style>
	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--sp-3);
		min-height: var(--control-h);
		padding: 0 var(--sp-4);
		border: var(--bw) solid transparent;
		border-radius: var(--radius);
		font-weight: var(--fw-medium);
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
		text-decoration: none;
		white-space: nowrap;
		cursor: pointer;
	}
	.btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.btn--primary {
		background: var(--action);
		color: var(--action-text);
		border-color: var(--action);
	}
	.btn--primary:not(:disabled):hover {
		background: var(--action-text);
		color: var(--action);
	}

	.btn--ghost {
		background: transparent;
		color: var(--text);
		border-color: var(--border-strong);
	}
	.btn--ghost:not(:disabled):hover {
		background: var(--action);
		color: var(--action-text);
	}

	.btn--danger {
		background: transparent;
		color: var(--danger);
		border-color: var(--danger);
	}
	.btn--danger:not(:disabled):hover {
		background: var(--danger);
		color: var(--danger-text);
	}

	/* Last: must win over `.btn:disabled`. Waiting is not "unavailable". */
	.btn--loading:disabled {
		opacity: 1;
		cursor: progress;
	}
</style>
