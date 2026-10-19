<script lang="ts">
	import { builds } from '$lib/instance';
	import type { LayerProps } from '$lib/plugins/plugin';
	import Drop from './Drop.svelte';
	import Panel from './Panel.svelte';
	import { parseHand, parseNear, parsePicked, parseRefused, type Edge, type Finish, type Item, type Mode, type NearOne, type Picked, type Refusal } from './protocol';

	// Items over the world: what the plugin says is in hand, which part is
	// picked and how it stands, and why it last refused. A front end only:
	// it shows those and asks for others, and what a click places and a
	// drag moves is the plugin's crate's. The menu is open while the plugin
	// says it is editing, from its button or from a key.
	let { seam, level, palette }: LayerProps = $props();

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
		});
		// What is in hand was said before this layer listened: ask for it.
		seam.command('state');
		return stop;
	});

	// The menu is one of the one UI open at a time: the stage hears when it
	// opens and closes it when another opens.
	$effect(() => seam.menu(editing));
	$effect(() => seam.closing(() => seam.command('edit', { on: false })));

	function ask(kind: string, body?: Record<string, unknown>) {
		refused = null;
		seam.command(kind, body);
	}
</script>

{#if builds(level)}
	<Drop {seam} {picked} />
	<Panel
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
		onlist={(on) => {
			listing = on;
			if (on && !editing) ask('edit', { on: true });
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
{/if}
