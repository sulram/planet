<script lang="ts">
	import { Select, Slider } from '$lib/ds';
	import type { KindProps } from './kinds';
	import { parseHand, parseLandmarks, type LandmarkOne } from './protocol';
	import { t } from './words';

	// The portal kind's own controls: where it leads, one of the landmarks
	// of the world, from the chart, and how wide its rim is. In hand, what
	// the next ring gets, given to the plugin with the item; with a portal
	// picked in the world, that one's, set as it is chosen. The chart is
	// asked for while the control shows, every two seconds, as the chart
	// panel asks.
	let { seam, picked }: KindProps = $props();

	let landmarks = $state<LandmarkOne[]>([]);
	/** The landmark chosen, by its name in the world; 0 is nowhere. */
	let to = $state(0);
	/** The rim's width, of the ring's width: 0 is none, 0.5 the middle. */
	let rim = $state(0.06);
	/** The ring's width in metres: the picked one's, or the four metres a hand places. */
	const width_m = $derived(picked ? picked.scale[0] * 0.5 : 4);

	const NOWHERE = '0';
	const options = $derived([
		{ value: NOWHERE, label: t('portal.nowhere') },
		...landmarks.map((one) => ({ value: String(one.id), label: one.name }))
	]);
	/** The chosen landmark may be gone from the chart: it is still shown, by its name in the world. */
	const value = $derived(options.some((option) => option.value === String(to)) ? String(to) : NOWHERE);

	$effect(() => {
		seam.command('chart');
		const every = setInterval(() => seam.command('chart'), 2000);
		return () => clearInterval(every);
	});

	$effect(() => {
		if (picked) {
			to = picked.to ?? 0;
			rim = picked.rim ?? rim;
		}
		const stop = seam.listen((kind, event) => {
			if (kind === 'landmarks') landmarks = parseLandmarks(event) ?? landmarks;
			if (kind !== 'hand' || picked) return;
			const hand = parseHand(event);
			if (hand?.item === 'portal') {
				to = hand.to ?? 0;
				rim = hand.rim;
			}
		});
		return stop;
	});

	function chosen(next: string) {
		to = Number(next);
		if (picked) seam.command('set', { to });
		else seam.command('take', { item: 'portal', to });
	}

	function rimmed(next: number) {
		rim = next;
		if (picked) seam.command('set', { rim });
		else seam.command('take', { item: 'portal', rim });
	}
</script>

<Select label={t('portal.to')} {options} {value} onselect={chosen} />
<p class="hint">{t('portal.to.hint')}</p>
<Slider
	label={t('portal.rim')}
	value={rim}
	min={0}
	max={0.5}
	step={0.005}
	format={(n) => (n === 0 ? t('portal.rim.none') : t('portal.rim.cm', { cm: (n * width_m * 100).toFixed(0) }))}
	oninput={rimmed}
/>

<style>
	.hint {
		color: var(--text-muted);
	}
</style>
