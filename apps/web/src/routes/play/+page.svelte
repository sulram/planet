<script lang="ts">
	import { enhance } from '$app/forms';
	import { goto } from '$app/navigation';
	import { Button, Field, Input, Segmented, Slider, Stack } from '$lib/ds';
	import Stage from '$lib/engine/Stage.svelte';
	import { t } from '$lib/i18n';
	import {
		buildParams,
		FIELD_PATH,
		knobsFor,
		normalizeSeed,
		randomSeed,
		SHAPES,
		WORLD_NAME_MAX,
		type Knobs,
		type Recipe,
		type Shape
	} from '$lib/world';
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
	// What the sliders hold: the page's knobs until one is dragged, and the
	// page's again whenever the URL brings new ones.
	// svelte-ignore state_referenced_locally
	let turned = $state<Knobs>({ ...data.knobs });
	$effect(() => {
		turned = { ...data.knobs };
	});

	const knobs = $derived(knobsFor(data.shape));
	const params = $derived(buildParams(data.shape, turned, data.field));
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

	// A planet is its whole address: seed, shape and every knob that is not at
	// its default. Changing the shape drops the knobs of the old one, because
	// they mean nothing to the new.
	function address(seed: string, shape: Shape): string {
		const query = new URLSearchParams({ seed, shape });
		if (shape === data.shape) {
			for (const knob of knobs) {
				const value = turned[knob.key] ?? knob.fallback;
				if (value !== knob.fallback) query.set(knob.key, value.toFixed(knob.places));
			}
		}
		return `/play?${query}`;
	}

	async function show(seed: string, shape: Shape = data.shape) {
		seedError = '';
		await goto(address(seed, shape), { keepFocus: true, noScroll: true });
	}

	// The sliders answer at once; the address catches up when one is let go,
	// so a planet stays shareable without a URL change on every frame.
	function keep() {
		history.replaceState(history.state, '', address(data.seed, data.shape));
	}

	function reset() {
		turned = Object.fromEntries(knobs.map((knob) => [knob.key, knob.fallback]));
		keep();
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
			{#each knobs as knob (knob.key)}
				<Slider
					label={t(`world.param.${knob.key}`)}
					value={turned[knob.key] ?? knob.fallback}
					min={knob.min}
					max={knob.max}
					step={knob.step}
					format={(n) => n.toFixed(knob.places)}
					oninput={(value) => (turned = { ...turned, [knob.key]: value })}
					onchange={keep}
				/>
				<input type="hidden" name={knob.key} value={turned[knob.key] ?? knob.fallback} form="create" />
			{/each}
			<Button type="submit">{t('play.regenerate')}</Button>
			<Button type="button" variant="ghost" onclick={() => show(randomSeed())}>{t('play.random')}</Button>
			<Button type="button" variant="ghost" onclick={reset}>{t('play.params.reset')}</Button>
		</Stack>
	</form>

	{#if data.user}
		<form
			method="POST"
			action="?/create&seed={data.seed}&shape={data.shape}"
			id="create"
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
