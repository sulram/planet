<script lang="ts">
	import { builds } from '$lib/instance';
	import type { LayerProps } from '$lib/plugins/plugin';
	import Panel from './Panel.svelte';
	import { parseHand, parseHistory, parseOver, parseRefused, parseStroke, type Base, type Edge, type Finish, type Refusal, type Tool } from './protocol';

	// Building over the world: what the plugin says is in hand, whether a
	// volume stands under the body, why it last refused, and what there is to
	// take back. A front end only: it shows
	// those and asks for others, and what a tool, a stroke and a platform do
	// is the plugin's crate's.
	let { seam, level, palette }: LayerProps = $props();

	let tool = $state<Tool | null>(null);
	let paint = $state(0);
	let finish = $state<Finish>('matte');
	let edge = $state<Edge>('none');
	let platform = $state(16);
	let volume = $state(false);
	let stroke = $state<[number, number, number] | null>(null);
	let refused = $state<Refusal | null>(null);
	let history = $state({ undo: false, redo: false });

	$effect(() => {
		const stop = seam.listen((kind, event) => {
			if (kind === 'hand') {
				const hand = parseHand(event);
				if (hand) ({ tool, paint, finish, edge, platform } = hand);
			} else if (kind === 'over') volume = parseOver(event) ?? volume;
			else if (kind === 'stroke') stroke = parseStroke(event);
			else if (kind === 'refused') refused = parseRefused(event);
			else if (kind === 'history') history = parseHistory(event) ?? history;
		});
		// What is in hand was said before this layer listened: ask for it.
		seam.command('state');
		return stop;
	});

	function take(next: Tool | null) {
		refused = null;
		seam.command('take', { tool: next });
	}

	function lay(base: Base) {
		refused = null;
		seam.command('lay', { base });
	}

	function closeVolume() {
		refused = null;
		seam.command('close');
	}
</script>

{#if builds(level)}
	<Panel
		{tool}
		{paint}
		{finish}
		{edge}
		{platform}
		{palette}
		{volume}
		{stroke}
		{refused}
		{history}
		onbuild={take}
		onpaint={(next) => seam.command('paint', { paint: next })}
		onfinish={(next) => seam.command('finish', { finish: next })}
		onedge={(next) => seam.command('edge', { edge: next })}
		onplatform={(side) => seam.command('platform', { side })}
		onlay={lay}
		onclosevolume={closeVolume}
		onundo={() => seam.command('undo')}
		onredo={() => seam.command('redo')}
	/>
{/if}
