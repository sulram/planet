<script lang="ts">
	import { Button } from '$lib/ds';
	import { KINDS, type Media } from './image';
	import type { KindProps } from './kinds';
	import { parseHand } from './protocol';
	import { notTaken, takeFile, type NotTaken } from './takeFile';
	import { t } from './words';

	// The picture kind's own controls: a file picked here is made into its
	// three versions, sent up, and taken in hand with the item, so the next
	// click places it; with a picture picked in the world, the file takes
	// that picture's place instead. A file dropped anywhere on the page goes
	// the same way (`Drop.svelte`). What is in hand is heard from the
	// plugin; what a placed picture is at every distance is the engine's
	// (`image.ts`).
	let { seam, picked }: KindProps = $props();

	/** The picture in hand: its record when it was given here, `true` when given elsewhere, `false` with none. */
	let picture = $state<Media | boolean>(false);
	let busy = $state(false);
	let error = $state<NotTaken | null>(null);
	let input = $state<HTMLInputElement>();

	$effect(() => {
		const stop = seam.listen((kind, event) => {
			if (kind !== 'hand') return;
			const hand = parseHand(event);
			if (hand) picture = hand.item === 'picture' ? picture || true : false;
		});
		return stop;
	});

	async function chosen(event: Event) {
		const file = (event.currentTarget as HTMLInputElement).files?.[0];
		if (!file) return;
		busy = true;
		error = null;
		try {
			const taken = await takeFile(seam, file, picked);
			if (taken.media && !picked) picture = taken.media;
		} catch (refused) {
			error = notTaken(refused);
		} finally {
			busy = false;
			if (input) input.value = '';
		}
	}

	function drop() {
		picture = false;
		seam.command('take', { item: 'picture', media: null });
	}
</script>

<div class="picture">
	<input bind:this={input} type="file" accept={KINDS.join(',')} onchange={chosen} hidden />
	{#if picked}
		<Button variant="ghost" loading={busy} onclick={() => input?.click()}>
			{busy ? t('picture.sending') : t('picture.replace')}
		</Button>
	{:else if picture}
		<span class="held">
			{typeof picture === 'object' ? t('picture.held', { w: String(picture.width), h: String(picture.height) }) : t('picture.inhand')}
		</span>
		<Button variant="ghost" onclick={drop}>{t('picture.another')}</Button>
	{:else}
		<Button variant="ghost" loading={busy} onclick={() => input?.click()}>
			{busy ? t('picture.sending') : t('picture.pick')}
		</Button>
	{/if}
	{#if error}<span class="error">{t(`picture.refused.${error}`)}</span>{/if}
</div>

<style>
	.picture {
		display: flex;
		gap: var(--sp-2);
		align-items: center;
		flex-wrap: wrap;
	}
	.held {
		color: var(--text-muted);
	}
	.error {
		color: var(--danger);
	}
</style>
