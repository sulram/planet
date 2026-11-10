<script lang="ts">
	import { Alert, Button, Spinner } from '$lib/ds';
	import { budget } from './budget';
	import type { KindProps } from './kinds';
	import { parseHand } from './protocol';
	import { notTakenOrUnfit, takeFile, type NotTaken, type Unfit } from './takeFile';
	import { t } from './words';

	// A model kind's own controls, of voxels or a 3D model by `kind`: a
	// `.vox` or a `.glb` chosen is sent to the world through the seam, a 3D
	// model weighed first, and the model it names is taken in hand; the next
	// click places it. A file dropped anywhere on the page goes the same way
	// (`Drop.svelte`). What is in hand is heard from the plugin, never kept
	// here.
	interface Props extends KindProps {
		kind?: 'model' | 'glb';
	}

	let { seam, kind = 'model' }: Props = $props();
	const accept = $derived(kind === 'glb' ? '.glb' : '.vox');

	let sending = $state(false);
	let refused = $state<NotTaken | Unfit | null>(null);
	let model = $state<string | null>(null);
	let loading = $state(false);
	let picker: HTMLInputElement | undefined = $state();

	$effect(() => {
		const stop = seam.listen((kind, event) => {
			if (kind !== 'hand') return;
			const hand = parseHand(event);
			if (hand) ({ model, loading } = hand);
		});
		seam.command('state');
		return stop;
	});

	async function send(file: File) {
		if (sending) return;
		refused = null;
		sending = true;
		try {
			await takeFile(seam, file);
		} catch (error) {
			refused = notTakenOrUnfit(error);
		} finally {
			sending = false;
		}
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

<div class="model" role="group" aria-label={t(`kind.${kind}`)}>
	{#if sending}
		<Spinner label={t('model.sending')} />
	{:else}
		<p>{t(`${kind}.drop`)}</p>
		<Button variant="ghost" onclick={() => picker?.click()}>{t('model.choose')}</Button>
		<input bind:this={picker} type="file" {accept} hidden onchange={chosen} />
		{#if model && loading}
			<p class="inhand"><Spinner label={t('model.loading')} /> {t('model.loading')}</p>
		{:else if model}
			<p class="inhand">{t('model.inhand', { name: short(model) })}</p>
		{/if}
	{/if}
	{#if refused}
		<Alert variant="info">{t(`drop.refused.${refused}`, budget())}</Alert>
	{/if}
</div>

<style>
	.model {
		display: flex;
		flex-direction: column;
		gap: var(--sp-2);
		color: var(--text-muted);
	}
	.inhand {
		color: var(--text);
		font-variant-numeric: tabular-nums;
	}
</style>
