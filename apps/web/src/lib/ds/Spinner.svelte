<script lang="ts">
	import { t } from '$lib/i18n';

	interface Props {
		/** Accessible name. */
		label?: string;
	}

	let { label = t('common.loading') }: Props = $props();
</script>

<span class="spinner" role="status" aria-label={label}></span>

<style>
	.spinner {
		display: inline-block;
		flex: none;
		width: var(--sp-3);
		height: var(--sp-3);
		border: var(--bw) solid currentColor;
		border-top-color: transparent;
		animation: turn 0.8s steps(8) infinite;
	}
	@keyframes turn {
		to {
			transform: rotate(360deg);
		}
	}
</style>
