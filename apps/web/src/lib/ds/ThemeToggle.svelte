<script lang="ts">
	import { t } from '$lib/i18n';
	import { theme } from './theme.svelte';
</script>

<button class="toggle" type="button" onclick={() => theme.toggle()} aria-label={t('common.theme.toggle')}>
	<svg viewBox="0 0 10 10" aria-hidden="true">
		<circle cx="5" cy="5" r="4" />
		<path d="M5 1a4 4 0 0 1 0 8z" />
	</svg>
</button>

<style>
	.toggle {
		display: inline-grid;
		place-items: center;
		width: var(--control-h);
		height: var(--control-h);
		border: var(--bw) solid var(--border-strong);
		background: transparent;
		color: var(--text);
		cursor: pointer;
	}
	.toggle:hover {
		background: var(--action);
		color: var(--action-text);
	}
	svg {
		width: var(--sp-4);
		height: var(--sp-4);
	}
	circle {
		fill: none;
		stroke: currentColor;
		stroke-width: var(--bw);
	}
	path {
		fill: currentColor;
	}
</style>
