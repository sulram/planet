<script lang="ts">
	import { Button } from '$lib/ds';
	import { upload } from '$lib/files';
	import type { Seam } from '$lib/plugins/plugin';
	import { KINDS, prepareImage, type Media } from './image';
	import { t } from './words';

	// A picture for the part tool: a file picked here is made into its three
	// versions, sent up, and put in hand. The next click places it. What a
	// placed picture is at every distance is the engine's (`image.ts`).
	let { seam, key }: { seam: Seam; key: string } = $props();

	let picture = $state<Media | null>(null);
	let busy = $state(false);
	let error = $state('');
	let input = $state<HTMLInputElement>();

	async function picked(event: Event) {
		const file = (event.currentTarget as HTMLInputElement).files?.[0];
		if (!file) return;
		busy = true;
		error = '';
		try {
			picture = await prepareImage(file, (blob) => upload(blob, key));
			seam.command('picture', { media: picture });
		} catch (refused) {
			error = refused instanceof Error ? refused.message : 'kind';
		} finally {
			busy = false;
			if (input) input.value = '';
		}
	}

	function drop() {
		picture = null;
		seam.command('picture', { media: null });
	}
</script>

<div class="picture">
	<input bind:this={input} type="file" accept={KINDS.join(',')} onchange={picked} hidden />
	{#if picture}
		<span class="held">{t('picture.held', { w: String(picture.width), h: String(picture.height) })}</span>
		<Button variant="ghost" onclick={drop}>{t('picture.drop')}</Button>
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
