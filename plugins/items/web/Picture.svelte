<script lang="ts">
	import { Button } from '$lib/ds';
	import { KINDS, prepareImage, type Media } from './image';
	import type { KindProps } from './kinds';
	import { t } from './words';

	// The picture kind's own controls: a file picked here is made into its
	// three versions, sent up, and taken in hand with the item, so the next
	// click places it; with a picture picked in the world, the file takes
	// that picture's place instead. What a placed picture is at every
	// distance is the engine's (`image.ts`).
	let { seam, picked }: KindProps = $props();

	/** Why the last file was not taken: the world's refusal codes, and the browser's. */
	type Refused = 'kind' | 'size' | 'level' | 'key' | 'unreachable' | 'canvas';
	const refusals: readonly Refused[] = ['kind', 'size', 'level', 'key', 'unreachable', 'canvas'];

	let picture = $state<Media | null>(null);
	let busy = $state(false);
	let error = $state<Refused | null>(null);
	let input = $state<HTMLInputElement>();

	async function chosen(event: Event) {
		const file = (event.currentTarget as HTMLInputElement).files?.[0];
		if (!file) return;
		busy = true;
		error = null;
		try {
			const media = await prepareImage(file, (blob) => seam.upload(blob));
			if (picked) {
				seam.command('set', { media });
			} else {
				picture = media;
				seam.command('take', { item: 'picture', media });
			}
		} catch (refused) {
			const code = refused instanceof Error ? refused.message : '';
			error = refusals.find((known) => known === code) ?? 'unreachable';
		} finally {
			busy = false;
			if (input) input.value = '';
		}
	}

	function drop() {
		picture = null;
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
		<span class="held">{t('picture.held', { w: String(picture.width), h: String(picture.height) })}</span>
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
