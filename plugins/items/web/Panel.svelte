<script lang="ts">
	import { Alert, Button, Checkbox, Icon, IconButton, Palette, Panel, Segmented, Slider, VectorInput, type IconName } from '$lib/ds';
	import { t as core } from '$lib/i18n';
	import type { Seam } from '$lib/plugins/plugin';
	import { kinds } from './kinds';
	import { edges, finishes, modes, type Edge, type Finish, type Item, type Mode, type NearOne, type Picked, type Refusal } from './protocol';
	import { t } from './words';

	// The menu of items: a button in the top left corner, and the panel it
	// opens. The kinds to place, the one in hand pressed; under them the
	// controls of that kind, when it has any. Then the item picked in the
	// world, when one is: what its handles do, where it stands, how it is
	// turned, how big it is, whether it is solid, and the way to remove it.
	// Then the material, the palette as building shows it, which paints what
	// is placed next and the item picked alike. The foot says what to do.
	interface Props {
		/** Whether the menu is open: the pointer picks items in the world. */
		editing: boolean;
		/** The item in hand, null when none. */
		item: Item | null;
		mode: Mode;
		picked: Picked | null;
		/** The part picked as it stood when it was picked: what reset goes back to. */
		origin: Picked | null;
		/** Index into `palette`. */
		paint: number;
		finish: Finish;
		edge: Edge;
		/** The paints a part can take, as `#rrggbb`, in order. */
		palette: string[];
		/** Why the last thing asked for was not done, until the next one. */
		refused: Refusal | null;
		seam: Seam;
		/** Whether the list of what is near is open, and what it says. */
		listing: boolean;
		near: NearOne[];
		onlist: (on: boolean) => void;
		onpick: (id: number) => void;
		onedit: (on: boolean) => void;
		ontake: (item: Item | null) => void;
		onmode: (mode: Mode) => void;
		onset: (set: Partial<Omit<Picked, 'id'>>) => void;
		onremove: () => void;
		onpaint: (paint: number) => void;
		onfinish: (finish: Finish) => void;
		onedge: (edge: Edge) => void;
	}

	let {
		editing,
		item,
		mode,
		picked,
		origin,
		paint,
		finish,
		edge,
		palette,
		refused,
		seam,
		listing,
		near,
		onlist,
		onpick,
		onedit,
		ontake,
		onmode,
		onset,
		onremove,
		onpaint,
		onfinish,
		onedge
	}: Props = $props();

	const modeIcons = { move: 'move-3d', rotate: 'rotate-3d', scale: 'scale-3d' } as const;
	const modeOptions = $derived(modes.map((value) => ({ value, label: t(`mode.${value}`) })));
	const finishOptions = $derived(finishes.map((value) => ({ value, label: t(`finish.${value}`) })));
	const edgeOptions = $derived(edges.map((value) => ({ value, label: t(`edge.${value}`) })));
	const inHand = $derived(kinds.find((kind) => kind.name === item));
	/** How far the list reaches, metres: a plot's width, as the plugin lists it. */
	const LIST_M = 32;
	const iconOf = (of: Item): IconName => kinds.find((kind) => kind.name === of)?.icon ?? 'box';
	const pickedKind = $derived(picked ? kinds.find((kind) => kind.name === picked.item) : undefined);
	const vector = (values: number[]) => [values[0], values[1], values[2]] as [number, number, number];
</script>

{#if editing}
	<Panel title={t('title')} corner="top-left">
		{#snippet aside()}
			<span class="head">
				<IconButton icon="list" label={t('near')} aria-expanded={listing} onclick={() => onlist(!listing)} />
				<IconButton icon="x" label="{core('common.close')} · Esc" onclick={() => onedit(false)} />
			</span>
		{/snippet}
		{#if listing}
			<section>
				<h3>{t('near')}</h3>
				{#if near.length === 0}
					<p class="muted">{t('near.empty', { m: String(LIST_M) })}</p>
				{:else}
					<ul class="near">
						{#each near as one (one.id)}
							<li>
								<button type="button" class="one" aria-pressed={picked?.id === one.id} onclick={() => onpick(one.id)}>
									<Icon name={iconOf(one.item)} />
									<span class="swatch" style:--paint={palette[one.paint % 16] ?? 'transparent'}></span>
									<span class="what">{kinds.find((kind) => kind.name === one.item)?.label() ?? one.item}</span>
									<span class="away">{t('near.away', { m: one.away_m.toFixed(1) })}</span>
								</button>
							</li>
						{/each}
					</ul>
				{/if}
			</section>
		{/if}
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
				<inHand.Extra {seam} />
			</section>
		{/if}
		{#if refused}
			<Alert variant="info">{t(`refused.${refused}`)}</Alert>
		{/if}
		{#if picked}
			<section>
				<header class="head">
					<h3>
						{#if pickedKind}<Icon name={pickedKind.icon} />{/if}
						{pickedKind ? t('picked', { kind: pickedKind.label() }) : t('part')}
					</h3>
					<Icon name={modeIcons[mode]} />
				</header>
				{#if pickedKind?.Extra}
					<pickedKind.Extra {seam} {picked} />
				{/if}
				<Segmented options={modeOptions} value={mode} label={t('mode')} onselect={onmode} />
				<VectorInput
					label={t('position')}
					axes={['u', 'v', 'h']}
					values={picked.at}
					step={0.5}
					decimals={1}
					onchange={(at) => onset({ at: vector(at) })}
					onreset={() => origin && onset({ at: origin.at })}
					resetLabel={t('reset')}
				/>
				<VectorInput
					label={t('rotation')}
					axes={['x', 'y', 'z']}
					values={picked.rotation}
					step={1}
					decimals={0}
					min={-180}
					max={180}
					onchange={(rotation) => onset({ rotation: vector(rotation) })}
					onreset={() => onset({ rotation: [0, 0, 0] })}
					resetLabel={t('reset')}
				/>
				<VectorInput
					label={t('scale')}
					axes={['x', 'y', 'z']}
					values={picked.scale}
					step={0.5}
					min={0.01}
					onchange={(scale) => onset({ scale: vector(scale) })}
					onreset={() => origin && onset({ scale: origin.scale })}
					resetLabel={t('reset')}
					lockLabel={t('lock')}
					locked
				/>
				<Checkbox checked={picked.solid} onchange={(solid) => onset({ solid })}>{t('solid')}</Checkbox>
				{#if picked.item === 'picture' || picked.lamp}
					<Slider label={t('shine')} value={picked.shine} min={0} max={8} step={0.1} format={(n) => n.toFixed(1)} oninput={(shine) => onset({ shine })} />
				{/if}
				{#if picked.lamp}
					<Slider
						label={t('reach')}
						value={picked.reach_m}
						min={0}
						max={128}
						step={1}
						format={(n) => (n === 0 ? t('reach.bySize') : t('near.away', { m: n.toFixed(0) }))}
						oninput={(reach_m) => onset({ reach_m })}
					/>
				{/if}
				<Button variant="danger" type="button" onclick={onremove}>{t('remove')} <kbd>Del</kbd></Button>
			</section>
		{/if}
		{#if item === 'cube' || (!item && picked?.item === 'cube')}
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
		{/if}
		<p class="foot">{t(item ? 'help.place' : picked ? 'help.edit' : 'help.pick')}</p>
	</Panel>
{:else}
	<div class="toggle">
		<IconButton icon="box" label="{t('title')} · I" onclick={() => onedit(true)} />
		<IconButton icon="list" label={t('near')} onclick={() => onlist(true)} />
	</div>
{/if}

<style>
	.toggle {
		--control-bg: var(--bg-overlay);
		position: absolute;
		top: var(--sp-4);
		left: var(--sp-4);
		z-index: var(--z-panel);
		display: flex;
		gap: var(--sp-2);
	}
	.head {
		display: flex;
		gap: var(--sp-1);
	}
	.muted {
		color: var(--text-muted);
	}
	.near {
		display: flex;
		flex-direction: column;
		gap: var(--sp-1);
	}
	/* A line of the list is the item's icon, its paint, its kind and how far
	   it is. */
	.one {
		display: grid;
		grid-template-columns: auto var(--sp-4) 1fr auto;
		align-items: center;
		gap: var(--sp-2);
		box-sizing: border-box;
		width: 100%;
		min-height: var(--control-h);
		padding: 0 var(--sp-2);
		border: var(--bw) solid transparent;
		background: transparent;
		color: var(--text);
		cursor: pointer;
		text-align: left;
	}
	.one:hover {
		border-color: var(--border-strong);
	}
	.one[aria-pressed='true'] {
		background: var(--action);
		color: var(--action-text);
	}
	.swatch {
		display: block;
		width: var(--sp-4);
		height: var(--sp-4);
		background: var(--paint);
	}
	.away {
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
	}
	.one[aria-pressed='true'] .away {
		color: var(--action-text);
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
	.kind:hover,
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
	.head {
		display: flex;
		justify-content: space-between;
		align-items: center;
		color: var(--text-muted);
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
	kbd {
		margin-left: var(--sp-2);
		opacity: 0.55;
	}
	.foot {
		padding-top: var(--sp-3);
		border-top: var(--bw) solid var(--border);
		color: var(--text-muted);
	}
</style>
