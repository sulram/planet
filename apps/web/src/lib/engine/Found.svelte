<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Alert, Button, Field, Input, Segmented, Slider, Stack } from '$lib/ds';
	import { readFieldSidecar } from '$lib/fields';
	import { t } from '$lib/i18n';
	import {
		buildParams,
		FIELD_PATH,
		fieldUrl,
		isShape,
		knobsFor,
		normalizeSeed,
		randomSeed,
		readKnobs,
		SHAPES,
		type Baked,
		type Field as Ground,
		type Knobs,
		type Recipe,
		type Shape
	} from '$lib/world';
	import Stage from './Stage.svelte';

	// The founding of a world (DECISIONS 87). Its admin walks candidate planets
	// in the offline preview and freezes the one on the screen. The seed, the
	// shape and the knobs live in the address, so a candidate can be shared
	// before it is chosen; arriving without a valid seed lands on a fresh one.
	interface Props {
		/** Asset path of the admin's avatar. */
		avatar: string | null;
		/** Why the last founding failed, as the server codes it. Empty when none did. */
		error?: string;
		/** A founding is on its way. */
		founding?: boolean;
		onfound: (recipe: Recipe) => void;
	}

	let { avatar, error = '', founding = false, onfound }: Props = $props();

	// The grounds this version has, read from their sidecars once. A recipe
	// names ground by the id read here, never by one typed or pasted: a world
	// may only ever be shaped by ground that is served.
	let grounds = $state.raw<Partial<Record<Baked, Ground>>>();
	onMount(async () => {
		const served: Partial<Record<Baked, Ground>> = {};
		for (const baked of Object.keys(FIELD_PATH) as Baked[]) {
			const ground = await readFieldSidecar(baked);
			if (ground) served[baked] = ground;
		}
		grounds = served;
	});

	const query = $derived(page.url.searchParams);
	/** The seed in the address, or null when it holds none in its canonical spelling. */
	const seed = $derived.by(() => {
		const asked = query.get('seed');
		return asked !== null && normalizeSeed(asked) === asked ? asked : null;
	});
	const shape = $derived.by<Shape>(() => {
		const asked = query.get('shape');
		return isShape(asked) && (asked === 'generated' || grounds?.[asked]) ? asked : 'generated';
	});
	const ground = $derived(shape === 'generated' ? null : (grounds?.[shape] ?? null));

	$effect(() => {
		if (seed) return;
		const next = normalizeSeed(query.get('seed')) ?? randomSeed();
		goto(`/?seed=${next}`, { replaceState: true, keepFocus: true, noScroll: true });
	});

	// What is typed in the field; the seed in the address is what is shown.
	let draft = $state('');
	$effect(() => {
		draft = seed ?? '';
	});
	let seedError = $state('');

	// The engine says which generator version it runs; the recipe is whole only
	// after that, so nothing is previewed or founded with a guessed version.
	let generatorVersion = $state<number>();
	// What the sliders hold: the address's knobs until one is dragged, and the
	// address's again whenever it brings new ones.
	const asked = $derived(readKnobs(shape, (key) => query.get(key)));
	let turned = $state<Knobs>({});
	$effect(() => {
		turned = { ...asked };
	});

	const knobs = $derived(knobsFor(shape));
	const recipe = $derived<Recipe | null>(
		seed && generatorVersion
			? { seed, generator_version: generatorVersion, params: buildParams(shape, turned, ground) }
			: null
	);
	const fieldPath = $derived(shape === 'generated' || !ground ? undefined : fieldUrl(shape, ground.id));

	const shapeOptions = $derived(SHAPES.map((value) => ({ value, label: t(`play.shape.${value}`) })));
	const shapeHint = $derived(
		shape === 'generated'
			? t('play.shape.generated.hint')
			: t('play.shape.earth.hint', { n: String(Math.round(ground?.texel_m ?? 0)) })
	);
	const failure = $derived(
		error === ''
			? ''
			: error === 'level'
				? t('found.error.level')
				: error === 'unreachable'
					? t('error.unreachable')
					: t('found.error.failed')
	);

	// A planet is its whole address: seed, shape and every knob that is not at
	// its default. Changing the shape drops the knobs of the old one, because
	// they mean nothing to the new.
	function address(nextSeed: string, nextShape: Shape): string {
		const next = new URLSearchParams({ seed: nextSeed, shape: nextShape });
		if (nextShape === shape) {
			for (const knob of knobs) {
				const value = turned[knob.key] ?? knob.fallback;
				if (value !== knob.fallback) next.set(knob.key, value.toFixed(knob.places));
			}
		}
		return `/?${next}`;
	}

	async function show(nextSeed: string, nextShape: Shape = shape) {
		seedError = '';
		await goto(address(nextSeed, nextShape), { keepFocus: true, noScroll: true });
	}

	// The sliders answer at once; the address catches up when one is let go,
	// so a planet stays shareable without a URL change on every frame.
	function settle() {
		if (seed) history.replaceState(history.state, '', address(seed, shape));
	}

	function reset() {
		turned = Object.fromEntries(knobs.map((knob) => [knob.key, knob.fallback]));
		settle();
	}

	function regenerate(event: SubmitEvent) {
		event.preventDefault();
		const next = normalizeSeed(draft);
		if (next) show(next);
		else seedError = t('play.seed.invalid');
	}
</script>

{#if grounds && seed}
	<Stage title={t('found.title')} {recipe} {fieldPath} {avatar} onready={(version) => (generatorVersion = version)}>
		<p>{t('found.lede')}</p>
		<form onsubmit={regenerate}>
			<Stack>
				<Field label={t('play.shape')} hint={shapeHint}>
					<Segmented options={shapeOptions} value={shape} label={t('play.shape')} onselect={(value) => show(seed, value)} />
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
						onchange={settle}
					/>
				{/each}
				<Button type="submit">{t('play.regenerate')}</Button>
				<Button type="button" variant="ghost" onclick={() => show(randomSeed())}>{t('play.random')}</Button>
				<Button type="button" variant="ghost" onclick={reset}>{t('play.params.reset')}</Button>
			</Stack>
		</form>
		<Stack>
			{#if failure}<Alert variant="danger">{failure}</Alert>{/if}
			<Field label={t('found.submit')} hint={t('found.hint')}>
				<Button type="button" loading={founding} disabled={!recipe} onclick={() => recipe && onfound(recipe)}>
					{t('found.submit')}
				</Button>
			</Field>
		</Stack>
	</Stage>
{/if}
