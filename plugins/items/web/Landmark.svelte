<script lang="ts">
	import { Field, Input } from '$lib/ds';
	import type { KindProps } from './kinds';
	import { parseHand } from './protocol';
	import { t } from './words';

	// The landmark kind's own control: its name. In hand, the name the next
	// star bears, given to the plugin with the item; with a landmark picked
	// in the world, the name of that one, set as it is typed. The bound and
	// what a name may hold are the part's (`part::Name`), and the plugin
	// says `name` when it refuses one.
	let { seam, picked }: KindProps = $props();

	let name = $state('');

	$effect(() => {
		if (picked) {
			name = picked.name ?? '';
			return;
		}
		const stop = seam.listen((kind, event) => {
			if (kind !== 'hand') return;
			const hand = parseHand(event);
			if (hand?.item === 'landmark' && hand.name !== null) name = hand.name;
		});
		return stop;
	});

	function changed() {
		if (name.trim() === '') return;
		if (picked) seam.command('set', { name });
		else seam.command('take', { item: 'landmark', name });
	}

	function keydown(event: KeyboardEvent) {
		if (event.key === 'Enter') (event.currentTarget as HTMLInputElement).blur();
	}
</script>

<Field label={t('landmark.name')} for="landmark-name" hint={t('landmark.name.hint')}>
	<Input id="landmark-name" bind:value={name} maxlength={64} placeholder={t('landmark.name.placeholder')} oninput={picked ? changed : undefined} onchange={changed} onkeydown={keydown} />
</Field>
