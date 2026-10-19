<script lang="ts">
	import { Button } from '$lib/ds';
	import type { KindProps } from './kinds';
	import { parseHand } from './protocol';
	import { notTaken, takeFile, type NotTaken } from './takeFile';
	import { t } from './words';

	// The sound kind's own controls: an audio file picked here is sent up
	// and placed in front of the body at once, looping, heard within its
	// reach and seen only by its gizmo. A file dropped anywhere on the page
	// goes the same way (`Drop.svelte`); the list of what is near finds a
	// placed sound again.
	let { seam }: KindProps = $props();

	let held = $state(false);
	let busy = $state(false);
	let error = $state<NotTaken | null>(null);
	let input = $state<HTMLInputElement>();

	$effect(() => {
		const stop = seam.listen((kind, event) => {
			if (kind !== 'hand') return;
			const hand = parseHand(event);
			if (hand) held = hand.item === 'sound';
		});
		return stop;
	});

	async function chosen(event: Event) {
		const file = (event.currentTarget as HTMLInputElement).files?.[0];
		if (!file) return;
		busy = true;
		error = null;
		try {
			await takeFile(seam, file);
		} catch (refused) {
			error = notTaken(refused);
		} finally {
			busy = false;
			if (input) input.value = '';
		}
	}
</script>

<div class="sound">
	<input bind:this={input} type="file" accept="audio/mpeg,audio/mp4,audio/x-m4a,audio/ogg,.mp3,.m4a,.ogg" onchange={chosen} hidden />
	<Button variant="ghost" loading={busy} onclick={() => input?.click()}>
		{busy ? t('sound.sending') : t('sound.pick')}
	</Button>
	{#if held}<span class="held">{t('sound.hint')}</span>{/if}
	{#if error}<span class="error">{t(`sound.refused.${error}`)}</span>{/if}
</div>

<style>
	.sound {
		display: flex;
		gap: var(--sp-2);
		align-items: center;
		flex-wrap: wrap;
	}
	.held {
		color: var(--fg-muted);
	}
	.error {
		color: var(--danger);
	}
</style>
