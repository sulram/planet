<script lang="ts">
	import { builds } from '$lib/instance';
	import type { LayerProps } from '$lib/plugins/plugin';
	import Panel from './Panel.svelte';
	import { parseHand, parsePicked, parseRefused, type Edge, type Finish, type Item, type Mode, type Picked, type Refusal } from './protocol';

	// Items over the world: what the plugin says is in hand, which part is
	// picked and how it stands, and why it last refused. A front end only:
	// it shows those and asks for others, and what a click places and a
	// drag moves is the plugin's crate's. The menu is open while the plugin
	// says it is editing, from its button or from a key.
	let { seam, level, palette, key }: LayerProps = $props();

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

{#if builds(level)}
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
		{key}
		onedit={(on) => ask('edit', { on })}
		ontake={(next) => ask('take', { item: next })}
		onmode={(next) => ask('mode', { mode: next })}
		onset={(set) => ask('set', set)}
		onremove={() => ask('remove')}
		onpaint={(next) => ask('paint', { paint: next })}
		onfinish={(next) => ask('finish', { finish: next })}
		onedge={(next) => ask('edge', { edge: next })}
	/>
{/if}
