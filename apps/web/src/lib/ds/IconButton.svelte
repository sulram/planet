<script lang="ts">
	import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';
	import Icon, { type IconName } from './Icon.svelte';

	// An action said by its icon alone, square and in the ghost look, for a
	// corner where words do not fit. With `href` it renders a link, so a
	// destination stays a real <a>. The label is required: it is the
	// accessible name and the tooltip, since an icon alone says nothing.
	interface Common {
		icon: IconName;
		label: string;
		/** A count or a mark in the corner of the icon, for what the button opens. */
		badge?: string;
	}
	type AsButton = Common & Omit<HTMLButtonAttributes, 'children'> & { href?: undefined };
	type AsLink = Common & Omit<HTMLAnchorAttributes, 'children'> & { href: string };
	type Props = AsButton | AsLink;

	let { icon, label, badge, ...rest }: Props = $props();
</script>

{#if rest.href !== undefined}
	<a class="icon-button" aria-label={label} title={label} {...rest as HTMLAnchorAttributes}>
		<Icon name={icon} />
	</a>
{:else}
	<!-- `type="button"` before the spread: a caller that submits says so -->
	<button class="icon-button" type="button" aria-label={label} title={label} {...rest as HTMLButtonAttributes}>
		<Icon name={icon} />
		{#if badge}<span class="badge" aria-hidden="true">{badge}</span>{/if}
	</button>
{/if}

<style>
	.icon-button {
		position: relative;
		display: inline-grid;
		place-items: center;
		width: var(--control-h);
		height: var(--control-h);
		border: var(--bw) solid var(--border-strong);
		background: transparent;
		color: var(--text);
		cursor: pointer;
	}
	.icon-button:not(:disabled):hover {
		background: var(--action);
		color: var(--action-text);
	}
	.icon-button:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.icon-button[aria-expanded='true'] {
		background: var(--action);
		color: var(--action-text);
	}
	.badge {
		position: absolute;
		top: calc(-1 * var(--bw));
		right: calc(-1 * var(--bw));
		min-width: var(--sp-4);
		padding-inline: var(--sp-1);
		line-height: var(--sp-4);
		background: var(--action);
		color: var(--action-text);
	}
</style>
