<script lang="ts">
	import { Alert, Icon, IconButton, Palette, Panel, Segmented } from '$lib/ds';
	import { t as core } from '$lib/i18n';
	import type { Seam } from '$lib/plugins/plugin';
	import { kinds } from './kinds';
	import { edges, finishes, type Edge, type Finish, type Item, type Refusal } from './protocol';
	import { t } from './words';

	// The menu of items: a button in the top left corner, and the panel it
	// opens. The kinds to place, the one in hand pressed; under them the
	// controls of that kind, when it has any, and the material what is placed
	// next is made of, the palette as building shows it. The foot says what
	// to do.
	interface Props {
		open: boolean;
		/** The item in hand, null when none. */
		item: Item | null;
		/** Index into `palette`. */
		paint: number;
		finish: Finish;
		edge: Edge;
		/** The paints a part can take, as `#rrggbb`, in order. */
		palette: string[];
		/** Why the last thing asked for was not done, until the next one. */
		refused: Refusal | null;
		seam: Seam;
		/** The session's key, for a kind that sends a file up. */
		key: string;
		onopen: () => void;
		onclose: () => void;
		ontake: (item: Item | null) => void;
		onpaint: (paint: number) => void;
		onfinish: (finish: Finish) => void;
		onedge: (edge: Edge) => void;
	}

	let { open, item, paint, finish, edge, palette, refused, seam, key, onopen, onclose, ontake, onpaint, onfinish, onedge }: Props = $props();

	const finishOptions = $derived(finishes.map((value) => ({ value, label: t(`finish.${value}`) })));
	const edgeOptions = $derived(edges.map((value) => ({ value, label: t(`edge.${value}`) })));
	const inHand = $derived(kinds.find((kind) => kind.name === item));
</script>

{#if open}
	<Panel title={t('title')} corner="top-left">
		{#snippet aside()}
			<IconButton icon="x" label="{core('common.close')} · Esc" onclick={onclose} />
		{/snippet}
		<div class="kinds" role="group" aria-label={t('kind')}>
			{#each kinds as kind (kind.name)}
				<button type="button" class="kind" aria-pressed={kind.name === item} onclick={() => ontake(kind.name === item ? null : kind.name)}>
					<Icon name={kind.icon} />
					<span>{kind.label()}</span>
				</button>
			{/each}
		</div>
		{#if inHand?.Extra}
			<section>
				<inHand.Extra {seam} {key} />
			</section>
		{/if}
		{#if refused}
			<Alert variant="info">{t(`refused.${refused}`)}</Alert>
		{/if}
		<section>
			<h3>{t('material')}</h3>
			<Palette colors={palette} {paint} {finish} {edge} label={t('paint')} pick={(n) => t('paint.pick', { n: String(n) })} onpick={onpaint} />
			<div class="row">
				<span>{t('finish')}</span>
				<Segmented options={finishOptions} value={finish} label={t('finish')} onselect={onfinish} />
			</div>
			<div class="row">
				<span>{t('edge')}</span>
				<Segmented options={edgeOptions} value={edge} label={t('edge')} onselect={onedge} />
			</div>
		</section>
		<p class="foot">{t(item ? 'help.place' : 'help.pick')}</p>
	</Panel>
{:else}
	<div class="toggle">
		<IconButton icon="box" label="{t('title')} · I" onclick={onopen} />
	</div>
{/if}

<style>
	.toggle {
		position: absolute;
		top: var(--sp-4);
		left: var(--sp-4);
		z-index: var(--z-panel);
	}
	.toggle {
		--control-bg: var(--bg-overlay);
	}
	.kinds {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: var(--sp-2);
	}
	/* A kind is its icon over its name, square, pressed while in hand. */
	.kind {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--sp-1);
		padding: var(--sp-3) var(--sp-2);
		border: var(--bw) solid var(--border-strong);
		background: transparent;
		color: var(--text);
		cursor: pointer;
	}
	.kind:hover {
		background: var(--action);
		color: var(--action-text);
	}
	.kind[aria-pressed='true'] {
		background: var(--action);
		color: var(--action-text);
	}
	section {
		display: flex;
		flex-direction: column;
		gap: var(--sp-3);
		padding-top: var(--sp-3);
		border-top: var(--bw) solid var(--border);
	}
	h3 {
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
		color: var(--text-muted);
	}
	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--sp-2);
		color: var(--text-muted);
	}
	.row > span {
		min-width: 11ch;
	}
	.foot {
		padding-top: var(--sp-3);
		border-top: var(--bw) solid var(--border);
		color: var(--text-muted);
	}
</style>
