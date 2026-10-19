<script lang="ts">
	import { Select } from '$lib/ds';
	import type { KindProps } from './kinds';
	import { parseHand, parseLandmarks, type LandmarkOne } from './protocol';
	import { t } from './words';

	// The portal kind's own control: where it leads, one of the landmarks of
	// the world, from the chart. In hand, the landmark the next door leads
	// to, given to the plugin with the item; with a portal picked in the
	// world, where that one leads, set as it is chosen. The chart is asked
	// for while the control shows, every two seconds, as the chart panel asks.
	let { seam, picked }: KindProps = $props();

	let landmarks = $state<LandmarkOne[]>([]);
	/** The landmark chosen, by its name in the world; 0 is nowhere. */
	let to = $state(0);

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
		}
		const stop = seam.listen((kind, event) => {
			if (kind === 'landmarks') landmarks = parseLandmarks(event) ?? landmarks;
			if (kind !== 'hand' || picked) return;
			const hand = parseHand(event);
			if (hand?.item === 'portal') to = hand.to ?? 0;
		});
		return stop;
	});

	function chosen(next: string) {
		to = Number(next);
		if (picked) seam.command('set', { to });
		else seam.command('take', { item: 'portal', to });
	}
</script>

<Select label={t('portal.to')} {options} {value} onselect={chosen} />
<p class="hint">{t('portal.to.hint')}</p>

<style>
	.hint {
		color: var(--text-muted);
	}
</style>
