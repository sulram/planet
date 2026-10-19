<script lang="ts">
	import { builds } from '$lib/instance';
	import type { LayerProps } from '$lib/plugins/plugin';
	import Panel from './Panel.svelte';
	import { parseHand, parseRefused, type Edge, type Finish, type Item, type Refusal } from './protocol';

	// Items over the world: what the plugin says is in hand and why it last
	// refused. A front end only: it shows those and asks for others, and
	// what a click places is the plugin's crate's. The menu opens from its
	// button, and opens itself when a key puts an item in hand.
	let { seam, level, palette }: LayerProps = $props();

	let open = $state(false);
	let item = $state<Item | null>(null);
	let paint = $state(0);
	let finish = $state<Finish>('matte');
	let edge = $state<Edge>('none');
	let refused = $state<Refusal | null>(null);

	$effect(() => {
		const stop = seam.listen((kind, event) => {
			if (kind === 'hand') {
				const hand = parseHand(event);
				if (!hand) return;
				({ item, paint, finish, edge } = hand);
				if (item) open = true;
			} else if (kind === 'refused') refused = parseRefused(event);
		});
		// What is in hand was said before this layer listened: ask for it.
		seam.command('state');
		return stop;
	});

	function take(next: Item | null) {
		refused = null;
		seam.command('take', { item: next });
	}

	function close() {
		open = false;
		if (item) take(null);
	}
</script>

{#if builds(level)}
	<Panel
		{open}
		{item}
		{paint}
		{finish}
		{edge}
		{palette}
		{refused}
		{seam}
		onopen={() => (open = true)}
		onclose={close}
		ontake={take}
		onpaint={(next) => seam.command('paint', { paint: next })}
		onfinish={(next) => seam.command('finish', { finish: next })}
		onedge={(next) => seam.command('edge', { edge: next })}
	/>
{/if}
