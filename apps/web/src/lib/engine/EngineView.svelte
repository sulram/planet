<script lang="ts">
	import { onMount } from 'svelte';
	import { Alert, Spinner } from '$lib/ds';
	import { t } from '$lib/i18n';
	import type { Recipe } from '$lib/world';
	import { loadEngine, type Effects, type Engine, type EngineEvent, type Mode } from './index';

	// Owns the canvas lifecycle: create on mount, free on destroy. The engine
	// runs its own frame loop, input listeners and resize tracking; this
	// component only keeps `recipe` and `mode` in sync through commands.
	interface Props {
		/** Null until the caller knows the whole recipe; nothing is sent meanwhile. */
		recipe: Recipe | null;
		mode: Mode;
		/** Asset path of the avatar to wear. Null wears any the manifest offers. */
		avatar: string | null;
		/** How the picture is made. Undefined leaves the engine as it is. */
		effects?: Effects;
		onevent?: (event: EngineEvent) => void;
	}

	let { recipe, mode, avatar, effects, onevent }: Props = $props();

	type Status = 'loading' | 'running' | 'missing' | 'unsupported' | 'failed';

	let canvas: HTMLCanvasElement | undefined = $state();
	let engine = $state.raw<Engine>();
	let status = $state<Status>('loading');

	// What the engine last reported or was last told: commands go out only on
	// a real difference, so an event echoed back by the parent sends nothing.
	let engineRecipe = '';
	let engineMode: Mode | undefined;
	let engineAvatar: string | undefined;
	let engineEffects = '';

	function receive(event: EngineEvent) {
		if (event.type === 'recipe_changed') engineRecipe = JSON.stringify(event.recipe);
		if (event.type === 'mode_changed') engineMode = event.mode;
		if (event.type === 'avatar_changed') engineAvatar = event.path;
		if (event.type === 'effects_changed') engineEffects = JSON.stringify(event.effects);
		onevent?.(event);
	}

	onMount(() => {
		let destroyed = false;
		let created: Engine | undefined;

		(async () => {
			const module = await loadEngine();
			if (destroyed) return;
			if (!module || !canvas) {
				status = 'missing';
				return;
			}
			try {
				created = await module.create(canvas, receive);
			} catch {
				status = 'gpu' in navigator ? 'failed' : 'unsupported';
				return;
			}
			if (destroyed) {
				created.free();
				return;
			}
			engine = created;
			status = 'running';
		})();

		return () => {
			destroyed = true;
			engine = undefined;
			created?.free();
		};
	});

	$effect(() => {
		if (!engine || !recipe) return;
		const next = JSON.stringify(recipe);
		if (next === engineRecipe) return;
		engineRecipe = next;
		engine.command({ type: 'set_recipe', recipe });
	});

	$effect(() => {
		if (!engine || (avatar && avatar === engineAvatar)) return;
		if (avatar) engineAvatar = avatar;
		engine.command(avatar ? { type: 'set_avatar', path: avatar } : { type: 'random_avatar' });
	});

	$effect(() => {
		if (!engine || !effects) return;
		const next = JSON.stringify(effects);
		if (next === engineEffects) return;
		engineEffects = next;
		engine.command({ type: 'set_effects', effects });
	});

	$effect(() => {
		if (!engine || mode === engineMode) return;
		engineMode = mode;
		engine.command({ type: 'set_mode', mode });
	});
</script>

<div class="view">
	<canvas bind:this={canvas} tabindex="0" aria-label={t('engine.canvas')} onclick={() => canvas?.focus()}></canvas>
	{#if status !== 'running'}
		<div class="notice">
			{#if status === 'loading'}
				<Spinner label={t('engine.loading')} />
			{:else if status === 'missing'}
				<Alert title={t('engine.missing.title')}>{t('engine.missing.body')} <code>bun run wasm</code></Alert>
			{:else if status === 'unsupported'}
				<Alert variant="danger" title={t('engine.unsupported.title')}>{t('engine.unsupported.body')}</Alert>
			{:else}
				<Alert variant="danger" title={t('engine.failed.title')}>{t('engine.failed.body')}</Alert>
			{/if}
		</div>
	{/if}
</div>

<style>
	.view {
		position: absolute;
		inset: 0;
		background: var(--bg-inset);
	}
	canvas {
		display: block;
		width: 100%;
		height: 100%;
		outline: none;
	}
	.notice {
		position: absolute;
		inset: 0;
		display: grid;
		place-items: center;
		padding: var(--sp-5);
		pointer-events: none;
	}
</style>
