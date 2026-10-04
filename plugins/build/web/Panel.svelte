<script lang="ts">
	import { Alert, Button, IconButton, Panel, Segmented } from '$lib/ds';
	import { t as core } from '$lib/i18n';
	import { bases, edges, finishes, platforms, tools, type Base, type Edge, type Finish, type Refusal, type Tool } from './protocol';
	import { t } from './words';

	// Building: a button in the bottom right corner, and the panel it opens
	// while a tool is in hand. It shows what its layer says is picked and asks
	// for another. What is always at hand sits in its head: take back, put
	// back, close. Under the tools, each with the key that takes it, comes
	// what the tool in hand works with. The volume tool: the size of the
	// platform, the volume under the body, to open with nothing in it or to
	// delete, and the bases a platform is laid on. Then the material a stroke
	// or a platform lays: the palette shows every colour in the finish and
	// with the edge picked, as it will be laid. The foot says how many cells
	// the stroke being drawn covers.
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
		/** Whether a volume stands under the body: one to delete, or none and one to open. */
		volume: boolean;
		/** Cells the stroke being drawn covers across, along and up. */
		stroke: [number, number, number] | null;
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
		/** Open the volume of the plot the body is over, with nothing built in it. */
		onopenvolume: () => void;
		/** Delete the volume the body is in, with all built in it. */
		onclosevolume: () => void;
		onundo: () => void;
		onredo: () => void;
	}

	let {
		tool,
		paint,
		finish,
		edge,
		platform,
		palette,
		volume,
		stroke,
		refused,
		history,
		onbuild,
		onpaint,
		onfinish,
		onedge,
		onplatform,
		onlay,
		onopenvolume,
		onclosevolume,
		onundo,
		onredo
	}: Props = $props();

	// Each tool with the key that takes it, in the order the keys run.
	const toolOptions = $derived(tools.map((value, index) => ({ value, label: t(`tool.${value}`), hint: String(index + 1) })));
	const finishOptions = $derived(finishes.map((value) => ({ value, label: t(`finish.${value}`) })));
	const edgeOptions = $derived(edges.map((value) => ({ value, label: t(`edge.${value}`) })));
	const platformOptions = platforms.map((side) => ({ value: String(side), label: String(side) }));

	// Building again takes the tool put down last, as B does in the engine:
	// the volume tool the first time.
	let last = $state<Tool>('volume');
	$effect(() => {
		if (tool) last = tool;
	});
</script>

{#if tool}
	<Panel title={t('title')} corner="bottom-right">
		{#snippet aside()}
			<div class="head">
				<IconButton icon="undo-2" label="{t('undo')} · {t('hint.undo.keys')}" disabled={!history.undo} onclick={onundo} />
				<IconButton icon="redo-2" label="{t('redo')} · {t('hint.redo.keys')}" disabled={!history.redo} onclick={onredo} />
				<IconButton icon="x" label="{core('common.close')} · Esc" onclick={() => onbuild(null)} />
			</div>
		{/snippet}
		<div class="tools">
			<Segmented options={toolOptions} value={tool} label={t('tool')} onselect={(value) => onbuild(value)} />
		</div>
		{#if tool === 'volume'}
			<section>
				<h3>{t('size')}</h3>
				<Segmented options={platformOptions} value={String(platform)} label={t('size')} onselect={(value) => onplatform(Number(value))} />
			</section>
			<section>
				<h3>{t('volume')}</h3>
				<div class="volume">
					{#if volume}
						<Button variant="danger" type="button" onclick={onclosevolume}>{t('close')}</Button>
					{:else}
						<Button variant="ghost" type="button" onclick={onopenvolume}>{t('open')}</Button>
					{/if}
				</div>
			</section>
			<section>
				<h3>{t('platform')}</h3>
				<div class="lay" role="group" aria-label={t('platform.lay')}>
					{#each bases as base (base)}
						<Button variant="ghost" type="button" onclick={() => onlay(base)}>{t(`lay.${base}`)}</Button>
					{/each}
				</div>
			</section>
		{/if}
		{#if refused}
			<Alert variant="info">{t(`refused.${refused}`)}</Alert>
		{/if}
		{#if tool !== 'delete'}
			<section>
				<h3>{t('material')}</h3>
				<div class="palette {finish} edge-{edge}" role="group" aria-label={t('paint')}>
					{#each palette as color, index (index)}
						<button
							type="button"
							class="swatch"
							style:--paint={color}
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
			</section>
		{/if}
		{#if stroke}
			<p class="foot count">
				{t('stroke.size', { x: stroke[0], y: stroke[1], z: stroke[2] })}
				<span>{t('stroke.cells', { n: stroke[0] * stroke[1] * stroke[2] })}</span>
			</p>
		{:else}
			<p class="foot">{t(tool === 'volume' ? 'help.volume' : 'help.stroke')}</p>
		{/if}
	</Panel>
{:else}
	<div class="toggle">
		<Button variant="ghost" type="button" onclick={() => onbuild(last)}>{t('title')} <kbd>B</kbd></Button>
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
	.toggle kbd {
		margin-left: var(--sp-2);
		opacity: 0.55;
	}
	.head {
		display: flex;
		gap: var(--sp-1);
	}
	/* Four tools, two to a row: they fit the panel in every language. */
	.tools :global(.segmented) {
		display: grid;
		grid-template-columns: 1fr 1fr;
		align-self: stretch;
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
	/* The bases that confirm a platform, side by side whatever their words. */
	.lay {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: var(--sp-2);
	}
	.lay :global(button) {
		padding-inline: 0;
	}
	/* The volume under the body: one thing to do with it, as wide as the panel. */
	.volume {
		display: grid;
	}
	.palette {
		display: grid;
		grid-template-columns: repeat(8, 1fr);
		gap: var(--sp-2);
	}
	/* A swatch is its paint as it will be laid: the colour in the finish and
	   with the edge picked. */
	.swatch {
		aspect-ratio: 1;
		border: none;
		background: var(--paint);
		cursor: pointer;
	}
	.glass .swatch {
		background: linear-gradient(135deg, color-mix(in srgb, var(--paint) 30%, transparent) 50%, color-mix(in srgb, var(--paint) 60%, transparent) 50%);
	}
	.light .swatch {
		box-shadow: 0 0 var(--sp-3) var(--paint);
	}
	.edge-black .swatch {
		box-shadow: inset 0 0 0 calc(2 * var(--bw)) black;
	}
	.edge-white .swatch {
		box-shadow: inset 0 0 0 calc(2 * var(--bw)) white;
	}
	.light.edge-black .swatch {
		box-shadow:
			inset 0 0 0 calc(2 * var(--bw)) black,
			0 0 var(--sp-3) var(--paint);
	}
	.light.edge-white .swatch {
		box-shadow:
			inset 0 0 0 calc(2 * var(--bw)) white,
			0 0 var(--sp-3) var(--paint);
	}
	.swatch[aria-pressed='true'] {
		outline: calc(2 * var(--bw)) solid var(--text);
		outline-offset: var(--bw);
	}
	.foot {
		padding-top: var(--sp-3);
		border-top: var(--bw) solid var(--border);
		color: var(--text-muted);
	}
	.count {
		display: flex;
		justify-content: space-between;
		color: var(--text);
	}
	.count span {
		color: var(--text-muted);
	}
</style>
