<script lang="ts">
	import { Alert, Button, Panel, Segmented } from '$lib/ds';
	import { t as core } from '$lib/i18n';
	import { bases, edges, finishes, platforms, tools, type Base, type Edge, type Finish, type Refusal, type Tool } from './protocol';
	import { t } from './words';

	// Building: a button in the bottom right corner, and the panel it opens
	// while a tool is in hand. It shows what its layer says is picked and asks
	// for another: the tool, and under it what that tool works with. The
	// platform tool has the size, the bases that confirm one, and the volume
	// to delete; a tool that lays cells has the colour, its finish and its
	// edge.
	interface Props {
		/** The tool in hand, null when not building. */
		tool: Tool | null;
		/** Index into `palette`. */
		paint: number;
		finish: Finish;
		edge: Edge;
		/** The side of the platform laid next, in cells. */
		platform: number;
		/** The paints a cell can take, as `#rrggbb`, in order. */
		palette: string[];
		/** Whether a volume stands under the body: one to delete. */
		volume: boolean;
		/** Why the last thing asked for was not done, until the next one. */
		refused: Refusal | null;
		/** Whether there is a change to take back, and one to put back. */
		history: { undo: boolean; redo: boolean };
		onbuild: (tool: Tool | null) => void;
		onpaint: (paint: number) => void;
		onfinish: (finish: Finish) => void;
		onedge: (edge: Edge) => void;
		onplatform: (side: number) => void;
		/** Lay a platform where the body stands, on a base. */
		onlay: (base: Base) => void;
		/** Delete the volume the body is in, with all built in it. */
		onclosevolume: () => void;
		onundo: () => void;
		onredo: () => void;
	}

	let { tool, paint, finish, edge, platform, palette, volume, refused, history, onbuild, onpaint, onfinish, onedge, onplatform, onlay, onclosevolume, onundo, onredo }: Props =
		$props();

	const toolOptions = $derived(tools.map((value) => ({ value, label: t(`tool.${value}`) })));
	const finishOptions = $derived(finishes.map((value) => ({ value, label: t(`finish.${value}`) })));
	const edgeOptions = $derived(edges.map((value) => ({ value, label: t(`edge.${value}`) })));
	const platformOptions = platforms.map((side) => ({ value: String(side), label: String(side) }));

	// Building again takes the tool put down last, as B does in the engine:
	// the platform tool the first time.
	let last = $state<Tool>('platform');
	$effect(() => {
		if (tool) last = tool;
	});
</script>

{#if tool}
	<Panel title={t('title')} corner="bottom-right">
		{#snippet aside()}
			<button class="close" type="button" onclick={() => onbuild(null)}>{core('common.close')}</button>
		{/snippet}
		<div class="tools">
			<Segmented options={toolOptions} value={tool} label={t('tool')} onselect={(value) => onbuild(value)} />
		</div>
		{#if tool === 'platform'}
			<div class="row">
				<span>{t('platform')}</span>
				<Segmented options={platformOptions} value={String(platform)} label={t('platform')} onselect={(value) => onplatform(Number(value))} />
			</div>
			<div class="row">
				{#each bases as base (base)}
					<Button variant="primary" type="button" onclick={() => onlay(base)}>{t(`lay.${base}`)}</Button>
				{/each}
			</div>
		{/if}
		{#if refused}
			<Alert variant="info">{t(`refused.${refused}`)}</Alert>
		{/if}
		{#if tool !== 'delete'}
			<div class="palette" role="group" aria-label={t('paint')}>
				{#each palette as color, index (index)}
					<button
						type="button"
						class="swatch"
						style:background={color}
						aria-pressed={index === paint}
						aria-label={t('paint.pick', { n: String(index + 1) })}
						onclick={() => onpaint(index)}
					></button>
				{/each}
			</div>
			<div class="row">
				<span>{t('finish')}</span>
				<Segmented options={finishOptions} value={finish} label={t('finish')} onselect={onfinish} />
			</div>
			<div class="row">
				<span>{t('edge')}</span>
				<Segmented options={edgeOptions} value={edge} label={t('edge')} onselect={onedge} />
			</div>
		{/if}
		<div class="row">
			<Button variant="ghost" type="button" disabled={!history.undo} onclick={onundo}>{t('undo')}</Button>
			<Button variant="ghost" type="button" disabled={!history.redo} onclick={onredo}>{t('redo')}</Button>
		</div>
		{#if tool === 'platform'}
			<div class="row">
				<Button variant="danger" type="button" disabled={!volume} onclick={onclosevolume}>{t('close')}</Button>
			</div>
		{/if}
		<p class="hint">{t(tool === 'platform' ? 'help.platform' : 'help.stroke')}</p>
	</Panel>
{:else}
	<div class="toggle">
		<Button variant="ghost" type="button" onclick={() => onbuild(last)}>{t('title')}</Button>
	</div>
{/if}

<style>
	.toggle {
		position: absolute;
		right: var(--sp-4);
		bottom: var(--sp-5);
		z-index: var(--z-panel);
	}
	.toggle :global(button) {
		background: var(--bg-overlay);
	}
	/* Four tools, two to a row: they fit the panel in every language. */
	.tools :global(.segmented) {
		display: grid;
		grid-template-columns: 1fr 1fr;
		align-self: stretch;
	}
	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--sp-2);
		color: var(--text-muted);
	}
	.close {
		border: none;
		background: none;
		color: var(--text-muted);
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
		cursor: pointer;
	}
	.close:hover {
		color: var(--text);
	}
	.palette {
		display: grid;
		grid-template-columns: repeat(8, 1fr);
		gap: var(--sp-1);
	}
	.swatch {
		aspect-ratio: 1;
		border: var(--bw) solid var(--border);
		cursor: pointer;
	}
	.swatch[aria-pressed='true'] {
		outline: var(--bw) solid var(--text);
		outline-offset: var(--bw);
	}
	.hint {
		color: var(--text-muted);
	}
</style>
