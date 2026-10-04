<script lang="ts">
	import { who } from '$lib/engine/who';
	import { t } from '$lib/i18n';
	import type { LayerProps } from '$lib/plugins/plugin';
	import Balloons from './Balloons.svelte';
	import Bar from './Bar.svelte';
	import { parseSaid, type Line, type Scope } from './protocol';

	// Chat over the world: the bar where a line is said, and a balloon over
	// the head that said it. Lines are what was heard while here, never
	// stored: the last hundred, each with the name its speaker had then.
	let { seam, online, me, peers, anchors }: LayerProps = $props();

	const LINES_KEPT = 100;
	let lines = $state<Line[]>([]);
	let lineCount = 0;

	$effect(() =>
		seam.listen((kind, event) => {
			const said = kind === 'said' ? parseSaid(event) : null;
			if (!said) return;
			const own = said.session === me;
			const peer = peers.find((p) => p.session === said.session);
			const line: Line = {
				...said,
				id: ++lineCount,
				who: own ? t('engine.here.you') : who(peer ?? { session: said.session, name: '', visitor: false }),
				own,
				at: Date.now()
			};
			lines = [...lines.slice(1 - LINES_KEPT), line];
		})
	);

	function say(scope: Scope, text: string, here: boolean) {
		seam.command('say', { scope, text, here });
	}
</script>

<Balloons {anchors} {me} {lines} />
<Bar {lines} {online} onsay={say} ongo={(place) => seam.core({ type: 'go_to', place })} onopen={seam.release} onclose={seam.take} />
