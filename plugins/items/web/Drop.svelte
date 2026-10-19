<script lang="ts">
	import { Icon, Spinner } from '$lib/ds';
	import type { Seam } from '$lib/plugins/plugin';
	import type { Picked } from './protocol';
	import { draggedOf, givenOf, notTaken, takeFile, type Given, type NotTaken } from './takeFile';
	import { t } from './words';

	// A file dragged anywhere over the page: an overlay over the world says
	// what dropping it does, a picture or a model by what the drag says of
	// it, and Esc cancels. Dropped, the file is taken as the item it is and
	// the next click places it; a file items takes no interest in is said
	// so, until Esc or a click.
	interface Props {
		seam: Seam;
		picked: Picked | null;
	}

	let { seam, picked }: Props = $props();

	type State = { over: Given | 'any' } | { sending: Given } | { refused: NotTaken } | null;
	let state = $state<State>(null);
	/** Nested enters and leaves of a drag over the page's elements, counted. */
	let depth = 0;
	/** Esc was pressed during this drag: the drop, when it comes, is let go. */
	let cancelled = false;

	const hasFiles = (event: DragEvent) => [...(event.dataTransfer?.types ?? [])].includes('Files');

	function enter(event: DragEvent) {
		if (!hasFiles(event)) return;
		event.preventDefault();
		depth += 1;
		if (depth === 1) {
			cancelled = false;
			state = { over: draggedOf([...(event.dataTransfer?.items ?? [])].map((item) => item.type)) };
		}
	}

	function over(event: DragEvent) {
		if (!hasFiles(event)) return;
		event.preventDefault();
		if (event.dataTransfer) event.dataTransfer.dropEffect = cancelled ? 'none' : 'copy';
	}

	function leave(event: DragEvent) {
		if (!hasFiles(event)) return;
		depth = Math.max(0, depth - 1);
		if (depth === 0 && state && 'over' in state) state = null;
	}

	async function drop(event: DragEvent) {
		if (!hasFiles(event)) return;
		event.preventDefault();
		depth = 0;
		const file = event.dataTransfer?.files[0];
		if (cancelled || !file) {
			state = null;
			return;
		}
		const given = givenOf(file.name, file.type);
		if (!given) {
			state = { refused: 'kind' };
			return;
		}
		state = { sending: given };
		try {
			await takeFile(seam, file, picked);
			state = null;
		} catch (error) {
			state = { refused: notTaken(error) };
		}
	}

	function key(event: KeyboardEvent) {
		if (event.key !== 'Escape' || !state) return;
		if ('over' in state) cancelled = true;
		state = null;
	}
</script>

<svelte:window ondragenter={enter} ondragover={over} ondragleave={leave} ondrop={drop} onkeydown={key} />

{#if state}
	<div class="drop" role="status">
		<div class="card">
			{#if 'over' in state}
				<Icon name={state.over === 'model' ? 'package' : 'image'} />
				<p class="says">{t(`drop.${state.over}`)}</p>
				<p class="esc">{t('drop.esc')}</p>
			{:else if 'sending' in state}
				<Spinner label={t('drop.sending')} />
				<p class="says">{t('drop.sending')}</p>
			{:else}
				<Icon name="x" />
				<p class="says">{t(`drop.refused.${state.refused}`)}</p>
				<button type="button" class="close" onclick={() => (state = null)}>{t('drop.close')}</button>
			{/if}
		</div>
	</div>
{/if}

<style>
	.drop {
		position: absolute;
		inset: 0;
		z-index: var(--z-topbar);
		display: grid;
		place-items: center;
		background: var(--backdrop);
	}
	/* A dashed field inside the edges of the page: anywhere in it is the drop. */
	.card {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--sp-3);
		padding: var(--sp-7) var(--sp-8);
		border: calc(2 * var(--bw)) dashed var(--on-world);
		color: var(--on-world);
		text-shadow: 0 0 var(--sp-2) var(--on-world-shadow);
		pointer-events: none;
	}
	.card :global(.icon) {
		width: var(--sp-8);
		height: var(--sp-8);
	}
	.says {
		font-weight: var(--fw-bold);
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
	}
	.esc {
		opacity: 0.7;
	}
	.close {
		pointer-events: auto;
		border: var(--bw) solid var(--on-world);
		background: transparent;
		color: var(--on-world);
		padding: var(--sp-2) var(--sp-4);
		cursor: pointer;
	}
</style>
