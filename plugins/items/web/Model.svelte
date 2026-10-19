<script lang="ts">
	import { Alert, Button, Spinner } from '$lib/ds';
	import { kindOf } from '$lib/files';
	import { Refused } from '$lib/instance';
	import type { KindProps } from './kinds';
	import { parseHand } from './protocol';
	import { t } from './words';

	// The model kind's own controls: a `.vox` file dropped or chosen is sent
	// to the world through the seam, and the model it names is taken in
	// hand; the next click places it. What is in hand is heard from the
	// plugin, never kept here.
	let { seam }: KindProps = $props();

	/** Why the world did not take the file, in the codes this panel has words for. */
	type Upload = 'kind' | 'size' | 'level' | 'key' | 'unreachable';
	const uploads: readonly Upload[] = ['kind', 'size', 'level', 'key', 'unreachable'];

	let sending = $state(false);
	let refused = $state<Upload | null>(null);
	let model = $state<string | null>(null);
	let picker: HTMLInputElement | undefined = $state();

	$effect(() => {
		const stop = seam.listen((kind, event) => {
			if (kind !== 'hand') return;
			const hand = parseHand(event);
			if (hand) model = hand.model;
		});
		seam.command('state');
		return stop;
	});

	async function send(file: Blob) {
		if (sending) return;
		refused = null;
		sending = true;
		try {
			const head = new Uint8Array(await file.slice(0, 16).arrayBuffer());
			if (kindOf(head) !== 'vox') throw new Refused('kind');
			const asset = await seam.upload(file);
			seam.command('take', { item: 'model', asset });
		} catch (error) {
			const code = error instanceof Refused ? error.code : '';
			refused = uploads.find((known) => known === code) ?? 'unreachable';
		} finally {
			sending = false;
		}
	}

	function dropped(event: DragEvent) {
		event.preventDefault();
		const file = event.dataTransfer?.files[0];
		if (file) void send(file);
	}

	function chosen(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (file) void send(file);
	}

	/** The model's name as a person reads it: the start of its hash. */
	const short = (asset: string) => asset.slice('asset://'.length, 'asset://'.length + 8);
</script>

<div class="model" role="group" aria-label={t('kind.model')} ondragover={(event) => event.preventDefault()} ondrop={dropped}>
	{#if sending}
		<Spinner label={t('model.sending')} />
	{:else}
		<p>{t('model.drop')}</p>
		<Button variant="ghost" onclick={() => picker?.click()}>{t('model.choose')}</Button>
		<input bind:this={picker} type="file" accept=".vox" hidden onchange={chosen} />
		{#if model}
			<p class="inhand">{t('model.inhand', { name: short(model) })}</p>
		{/if}
	{/if}
	{#if refused}
		<Alert variant="info">{t(`upload.${refused}`)}</Alert>
	{/if}
</div>

<style>
	.model {
		display: flex;
		flex-direction: column;
		gap: var(--sp-2);
		padding: var(--sp-3);
		border: var(--bw) dashed var(--border-strong);
		color: var(--text-muted);
	}
	.inhand {
		color: var(--text);
		font-variant-numeric: tabular-nums;
	}
</style>
