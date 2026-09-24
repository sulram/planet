<script lang="ts">
	import { onMount } from 'svelte';
	import { Alert, Spinner } from '$lib/ds';
	import { t } from '$lib/i18n';
	import { fieldId, type Recipe } from '$lib/world';
	import type { Link } from '$lib/server/session';
	import { loadEngine, type Effects, type Engine, type EngineEvent, type Mode } from './index';

	// Owns the canvas lifecycle: create on mount, free on destroy. The engine
	// runs its own frame loop, input listeners and resize tracking; this
	// component only keeps `recipe` and `mode` in sync through commands.
	interface Props {
		/** Null until the caller knows the whole recipe; nothing is sent meanwhile. */
		recipe: Recipe | null;
		/** Where to fetch the field a recipe names, when it names one. */
		fieldPath?: string;
		mode: Mode;
		/** Asset path of the avatar to wear. Null wears any the manifest offers. */
		avatar: string | null;
		/** How the picture is made. Undefined leaves the engine as it is. */
		effects?: Effects;
		/**
		 * The place from the address bar, for the world that link was for.
		 *
		 * Sent once, on the world the page asked for. Keeping your footing
		 * across a later recipe is the engine's own job, because only it knows
		 * what the new ground puts under an address: a place carried over by
		 * the page lands you in the sea every time the coastline moves.
		 */
		stand?: string | null;
		/**
		 * The world's socket, when this is a world and not a preview. The
		 * engine connects once it stands in the recipe, and connects again
		 * with a fresh ticket whenever the link drops.
		 */
		link?: Link;
		onevent?: (event: EngineEvent) => void;
	}

	let { recipe, fieldPath, mode, avatar, effects, stand, link, onevent }: Props = $props();

	type Status = 'loading' | 'shaping' | 'running' | 'missing' | 'unsupported' | 'failed';

	let canvas: HTMLCanvasElement | undefined = $state();
	let engine = $state.raw<Engine>();
	let status = $state<Status>('loading');

	// What the engine last reported or was last told: commands go out only on
	// a real difference, so an event echoed back by the parent sends nothing.
	let engineRecipe = '';
	let engineMode: Mode | undefined;
	let engineAvatar: string | undefined;
	let engineEffects = '';
	/** The field already handed to the engine, by id. */
	let engineField: string | undefined;
	/** Whether the arrival place has been honoured. It is good for one world. */
	let arrived = false;

	// The link: opened once the engine stands in the world the page wants,
	// opened again after a drop, each time with a fresh ticket. The wait
	// doubles from a second to half a minute, and a welcome resets it.
	let linked = false;
	let retryMs = 1000;
	let retry: ReturnType<typeof setTimeout> | undefined;

	async function connect() {
		retry = undefined;
		const here = engine;
		if (!here || !link || !linked) return;
		let url = link.url;
		if (link.ticketPath) {
			const ticket = await fetch(link.ticketPath, { method: 'POST' })
				.then((response) => (response.ok ? response.json() : null))
				.then((body: { ticket?: string } | null) => body?.ticket)
				.catch(() => undefined);
			if (here !== engine || !linked) return;
			if (ticket) url += `?ticket=${encodeURIComponent(ticket)}`;
		}
		here.connect(url);
	}

	function receive(event: EngineEvent) {
		if (event.type === 'recipe_changed') {
			engineRecipe = JSON.stringify(event.recipe);
			// Whether the engine now stands in the world the page wanted. It
			// builds a world of its own before the page's arrives, and the
			// page's waits on a fetch when a field shapes it, so the first
			// world reported is not the one asked for. The engine says the
			// recipe in full and the page in what was chosen, so the seed and
			// the version are what the two agree on; the engine checks the
			// whole recipe against the server's on arrival.
			const wanted =
				recipe && event.recipe.seed === recipe.seed && event.recipe.generator_version === recipe.generator_version;
			// The address bar's place is honoured once, for the world its link
			// was written for. Every world after it is new ground, where those
			// characters name somewhere else, so the engine's own respawn is
			// the one that knows better.
			if (wanted && !arrived && stand && engine) {
				arrived = true;
				engine.command({ type: 'go_to', place: stand });
			}
			if (wanted && link && !linked) {
				linked = true;
				connect();
			}
		}
		if (event.type === 'session') {
			if (event.status === 'online') retryMs = 1000;
			if (event.status === 'offline' && linked && !retry) {
				retry = setTimeout(connect, retryMs);
				retryMs = Math.min(retryMs * 2, 30_000);
			}
		}
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
			linked = false;
			clearTimeout(retry);
			engine = undefined;
			created?.free();
		};
	});

	// A world shaped by a field cannot be generated without it, so the bytes
	// go over first and the recipe follows. The engine keeps the field, so a
	// new seed over the same ground downloads nothing.
	$effect(() => {
		if (!engine || !recipe) return;
		const next = JSON.stringify(recipe);
		if (next === engineRecipe) return;
		const wanted = fieldId(recipe);
		const here = engine;
		if (!wanted || wanted === engineField) {
			engineRecipe = next;
			here.command({ type: 'set_recipe', recipe });
			return;
		}
		if (!fieldPath) return;
		engineRecipe = next;
		status = 'shaping';
		fetch(fieldPath)
			.then((response) => (response.ok ? response.arrayBuffer() : Promise.reject(response.status)))
			.then((bytes) => {
				if (here !== engine) return;
				here.set_field(new Uint8Array(bytes));
				engineField = wanted;
				status = 'running';
				here.command({ type: 'set_recipe', recipe });
			})
			.catch(() => {
				engineRecipe = '';
				status = 'failed';
			});
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
			{:else if status === 'shaping'}
				<Spinner label={t('engine.shaping')} />
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
