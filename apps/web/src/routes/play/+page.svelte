<script lang="ts">
	import { enhance } from '$app/forms';
	import { goto } from '$app/navigation';
	import { Button, Field, Input, Segmented, Stack } from '$lib/ds';
	import Stage from '$lib/engine/Stage.svelte';
	import { t } from '$lib/i18n';
	import { FIELD_PATH, normalizeSeed, randomSeed, SHAPES, WORLD_NAME_MAX, type Recipe, type Shape } from '$lib/world';
	import type { PageProps } from './$types';

	let { data, form }: PageProps = $props();

	// What is typed in the field; `data.seed` (from the URL) is what is shown.
	// svelte-ignore state_referenced_locally
	let draft = $state(data.seed);
	let seedError = $state('');
	let creating = $state(false);

	// The engine says which generator version it runs; the recipe is whole only
	// after that, so nothing is previewed or saved with a guessed version.
	let generatorVersion = $state<number>();
	const params = $derived(data.field ? { source: { field: data.field.id } } : {});
	const recipe = $derived<Recipe | null>(
		generatorVersion ? { seed: data.seed, generator_version: generatorVersion, params } : null
	);
	const fieldPath = $derived(data.shape === 'generated' ? undefined : FIELD_PATH[data.shape]);

	const shapeOptions = $derived(SHAPES.map((value) => ({ value, label: t(`play.shape.${value}`) })));
	const shapeHint = $derived(
		data.shape === 'generated'
			? t('play.shape.generated.hint')
			: t('play.shape.earth.hint', { n: String(Math.round(data.field?.texel_m ?? 0)) })
	);

	const playHref = $derived(`/play?seed=${data.seed}&shape=${data.shape}`);

	$effect(() => {
		draft = data.seed;
	});

	async function show(seed: string, shape: Shape = data.shape) {
		seedError = '';
		await goto(`/play?seed=${seed}&shape=${shape}`, { keepFocus: true, noScroll: true });
	}

	function regenerate(event: SubmitEvent) {
		event.preventDefault();
		const seed = normalizeSeed(draft);
		if (seed) show(seed);
		else seedError = t('play.seed.invalid');
	}
</script>

<svelte:head>
	<title>{t('play.title')} · {t('common.appName')}</title>
</svelte:head>

<Stage title={t('play.title')} {recipe} {fieldPath} avatar={data.avatar} onready={(version) => (generatorVersion = version)}>
	<form method="GET" action="/play" onsubmit={regenerate}>
		<Stack>
			<Field label={t('play.shape')} hint={shapeHint}>
				<Segmented
					options={shapeOptions}
					value={data.shape}
					label={t('play.shape')}
					onselect={(value) => show(data.seed, value)}
				/>
			</Field>
			<Field label={t('world.seed')} for="seed" hint={t('play.seed.hint')} error={seedError}>
				<Input
					id="seed"
					name="seed"
					bind:value={draft}
					maxlength={18}
					autocomplete="off"
					autocapitalize="off"
					spellcheck="false"
					invalid={!!seedError}
				/>
			</Field>
			<Button type="submit">{t('play.regenerate')}</Button>
			<Button type="button" variant="ghost" onclick={() => show(randomSeed())}>{t('play.random')}</Button>
		</Stack>
	</form>

	{#if data.user}
		<form
			method="POST"
			action="?/create&seed={data.seed}&shape={data.shape}"
			use:enhance={() => {
				creating = true;
				return async ({ update }) => {
					await update({ reset: false });
					creating = false;
				};
			}}
		>
			<input type="hidden" name="seed" value={data.seed} />
			<input type="hidden" name="shape" value={data.shape} />
			<input type="hidden" name="generator_version" value={generatorVersion ?? ''} />
			<Stack>
				<Field label={t('world.name')} for="name" hint={t('play.create.hint')} error={form?.error}>
					<Input
						id="name"
						name="name"
						value={form?.name ?? ''}
						maxlength={WORLD_NAME_MAX}
						autocomplete="off"
						invalid={!!form?.error}
						required
					/>
				</Field>
				<Button type="submit" loading={creating} disabled={!recipe}>{t('play.create')}</Button>
			</Stack>
		</form>
	{:else}
		<p class="signin">
			<a href="/login?redirect={encodeURIComponent(playHref)}">{t('play.signin')}</a>
		</p>
	{/if}
</Stage>

<style>
	.signin {
		color: var(--text-muted);
	}
</style>
