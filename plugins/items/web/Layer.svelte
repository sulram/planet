<script lang="ts">
	import { builds } from '$lib/instance';
	import type { LayerProps } from '$lib/plugins/plugin';
	import Drop from './Drop.svelte';
	import Panel from './Panel.svelte';
	import { parseHand, parseLandmarks, parseNear, parsePicked, parseRefused, type Edge, type Finish, type Item, type LandmarkOne, type Mode, type NearOne, type Picked, type Refusal } from './protocol';

	// Items over the world: what the plugin says is in hand, which part is
	// picked and how it stands, and why it last refused. A front end only:
	// it shows those and asks for others, and what a click places and a
	// drag moves is the plugin's crate's. The menu is open while the plugin
	// says it is editing, from its button or from a key. The chart, the
	// landmarks of the world, is anyone's: a line travels there through the
	// engine's own `go_to`, as a shared link does.
	let { seam, level, palette }: LayerProps = $props();

	const builder = $derived(builds(level));

	let editing = $state(false);
	let item = $state<Item | null>(null);
	let mode = $state<Mode>('move');
	let picked = $state<Picked | null>(null);
	/** The part picked as it stood when it was picked: what reset goes back to. */
	let origin = $state<Picked | null>(null);
	let paint = $state(0);
	let finish = $state<Finish>('matte');
	let edge = $state<Edge>('none');
	let refused = $state<Refusal | null>(null);
	/** Whether the list of what is near is open, and what it says. */
	let listing = $state(false);
	let near = $state<NearOne[]>([]);
	/** Whether the chart is open, and what it says. */
	let charting = $state(false);
	let landmarks = $state<LandmarkOne[]>([]);

	// Open, the chart is asked for at once and then every two seconds:
	// others place, and the body moves.
	$effect(() => {
		if (!charting) return;
		seam.command('chart');
		const every = setInterval(() => seam.command('chart'), 2000);
		return () => clearInterval(every);
	});

	// Open, the list is asked for at once and then every second: the body
	// moves and others place.
	$effect(() => {
		if (!listing) return;
		seam.command('list');
		const every = setInterval(() => seam.command('list'), 1000);
		return () => clearInterval(every);
	});

	$effect(() => {
		const stop = seam.listen((kind, event) => {
			if (kind === 'hand') {
				const hand = parseHand(event);
				if (hand) ({ editing, item, mode, paint, finish, edge } = hand);
			} else if (kind === 'picked') {
				picked = parsePicked(event);
				if (picked?.id !== origin?.id) origin = picked;
			}
			else if (kind === 'refused') refused = parseRefused(event);
			else if (kind === 'near') near = parseNear(event) ?? near;
			else if (kind === 'landmarks') landmarks = parseLandmarks(event) ?? landmarks;
		});
		// What is in hand was said before this layer listened: ask for it.
		seam.command('state');
		return stop;
	});

	function ask(kind: string, body?: Record<string, unknown>) {
		refused = null;
		seam.command(kind, body);
	}
</script>

{#if builder}
	<Drop {seam} {picked} />
{/if}
<Panel
	{builder}
	{editing}
	{item}
	{mode}
	{picked}
	{origin}
	{paint}
	{finish}
	{edge}
	{palette}
	{refused}
	{seam}
	{listing}
	{near}
	{charting}
	{landmarks}
	onlist={(on) => {
		listing = on;
		if (on && !editing) ask('edit', { on: true });
	}}
	onchart={(on) => (charting = on)}
	ongo={(place) => {
		charting = false;
		seam.core({ type: 'go_to', place });
	}}
	onadd={() => item && ask('take', { item, ahead: true })}
	onclose={() => {
		charting = false;
		listing = false;
		if (editing) ask('edit', { on: false });
	}}
	onpick={(id) => ask('pick', { id })}
	onedit={(on) => {
		if (!on) listing = false;
		ask('edit', { on });
	}}
	ontake={(next) => ask('take', { item: next })}
	onmode={(next) => ask('mode', { mode: next })}
	onset={(set) => ask('set', set)}
	onremove={() => ask('remove')}
	onpaint={(next) => ask('paint', { paint: next })}
	onfinish={(next) => ask('finish', { finish: next })}
	onedge={(next) => ask('edge', { edge: next })}
/>
