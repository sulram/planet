<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { Alert, Spinner } from '$lib/ds';
	import { t } from '$lib/i18n';
	import { fieldId, type Recipe } from '$lib/world';
	import type { Link } from '$lib/instance';
	import { loadEngine, type Command, type Effects, type Engine, type EngineEvent, type Mode } from './index';

	// Owns the canvas lifecycle: create on mount, free on destroy. The engine
	// runs its own frame loop, input listeners and resize tracking; this
	// component keeps `recipe` and `mode` in sync through commands, and keeps
	// a veil over the picture until it is the world the page asked for.
	interface Props {
		/** Null until the caller knows the whole recipe; nothing is sent meanwhile. */
		recipe: Recipe | null;
		/** Where to fetch the field a recipe names, when it names one. */
		fieldPath?: string;
		mode: Mode;
		/** Asset path of the avatar to wear. Null wears any the manifest offers. */
		avatar: string | null;
		/** What to be called. Sent before the link opens, and again whenever it changes. */
		name?: string;
		/** How the picture is made. Undefined leaves the engine as it is. */
		effects?: Effects;
		/**
		 * The address bar's place, each time a hand other than the page's puts
		 * one there: the link that brought you here, a place pasted over it, a
		 * world entered from another. A new object each time, since the same
		 * place asked for twice is two walks. Honoured once the engine stands
		 * in the world the page asked for, so a place meant for a world you
		 * are not yet in waits for it. Where you stand after that is the
		 * engine's: a place carried over a recipe change by the page lands you
		 * in the sea every time the coastline moves (DECISIONS 62, 73).
		 */
		stand?: { place: string } | null;
		/**
		 * The world's socket, when this is a world and not a preview. The
		 * engine connects once it stands in the recipe, and connects again
		 * whenever the link drops.
		 */
		link?: Link;
		onevent?: (event: EngineEvent) => void;
	}

	let { recipe, fieldPath, mode, avatar, name = '', effects, stand, link, onevent }: Props = $props();

	/**
	 * A command with no prop of its own: a line said, a place gone to. Dropped
	 * while the engine is not running, as every command is until it runs.
	 */
	export function command(command: Command) {
		engine?.command(command);
	}

	/** Hands the world back: the canvas takes focus and the pointer. */
	export function take() {
		canvas?.focus();
		canvas?.requestPointerLock();
	}

	/** Lets the world go, so a form can have the keys and the pointer. */
	export function release() {
		if (document.pointerLockElement === canvas) document.exitPointerLock();
	}

	/** The engine's own life, before any world is asked of it. */
	type Phase = 'loading' | 'ready' | 'missing' | 'unsupported' | 'failed';
	/** What the veil says while it is down, and `running` once it lifts. */
	type Status = Exclude<Phase, 'ready'> | 'shaping' | 'entering' | 'unshaped' | 'refused' | 'running';

	let canvas: HTMLCanvasElement | undefined = $state();
	let engine = $state.raw<Engine>();
	let phase = $state<Phase>('loading');
	/** The field is on its way. */
	let fetching = $state(false);
	/**
	 * The engine stands in the world the page asked for. False from the moment
	 * a recipe is sent until the engine says it stands in it.
	 */
	let standing = $state(false);
	/** That world is drawn whole where you stand: the streamer has nothing left to build. */
	let settled = $state(false);
	/** The recipe names ground this instance does not serve. */
	let unshaped = $state(false);
	/** The engine refused the recipe the page asked for. */
	let refused = $state(false);
	// The veil stays down until the picture is the world the page asked for,
	// drawn: the engine builds a world of its own before the page's arrives,
	// the page's waits on a fetch when a field shapes it, and its ground takes
	// a moment to stream in where you land.
	const status = $derived<Status>(
		phase !== 'ready'
			? phase
			: unshaped
				? 'unshaped'
				: refused
					? 'refused'
					: fetching
						? 'shaping'
						: recipe && !(standing && settled)
							? 'entering'
							: 'running'
	);

	// What the engine last reported or was last told: commands go out only on
	// a real difference, so an event echoed back by the parent sends nothing.
	let engineRecipe = '';
	let engineMode: Mode | undefined;
	let engineAvatar: string | undefined;
	let engineName: string | undefined;
	let engineEffects = '';
	/** The field already handed to the engine, by id. */
	let engineField: string | undefined;
	/** A place from the address bar, waiting for the world it was written for. */
	let pending: string | null = null;

	function arrive() {
		if (!engine || !standing || !pending) return;
		engine.command({ type: 'go_to', place: pending });
		pending = null;
	}

	// The link: opened once the engine stands in the world the page wants,
	// opened again after a drop, once the page has said its key still stands.
	// The wait doubles from a second to half a minute, and a welcome resets it.
	let linked = false;
	let retryMs = 1000;
	let retry: ReturnType<typeof setTimeout> | undefined;

	async function connect(again = false) {
		retry = undefined;
		const here = engine;
		if (!here || !link || !linked) return;
		if (again) {
			await link.again?.();
			if (here !== engine || !linked) return;
		}
		here.connect(link.url);
	}

	function receive(event: EngineEvent) {
		if (event.type === 'recipe_changed') {
			engineRecipe = JSON.stringify(event.recipe);
			// Whether the engine now stands in the world the page wanted. The
			// engine says the recipe in full and the page in what was chosen,
			// so the seed and the version are what the two agree on; the
			// engine checks the whole recipe against the server's on arrival.
			// A world the engine changed on its own, a new seed at the keys,
			// is not one the page awaits: it lifts no veil and drops none.
			const wanted =
				recipe && event.recipe.seed === recipe.seed && event.recipe.generator_version === recipe.generator_version;
			if (wanted) {
				standing = true;
				settled = false;
				arrive();
				if (link && !linked) {
					linked = true;
					connect();
				}
			}
		}
		// The ground is drawn where you stand. Heard for the engine's own world
		// too, which is not the one awaited.
		if (event.type === 'settled' && standing) settled = true;
		// While a world is awaited, its recipe is the only command in flight
		// that can be refused.
		if (event.type === 'rejected' && recipe && !standing && !fetching) refused = true;
		if (event.type === 'session') {
			if (event.status === 'online') retryMs = 1000;
			if (event.status === 'offline' && linked && !retry) {
				retry = setTimeout(() => connect(true), retryMs);
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
				phase = 'missing';
				return;
			}
			try {
				created = await module.create(canvas, receive);
			} catch {
				phase = 'gpu' in navigator ? 'failed' : 'unsupported';
				return;
			}
			if (destroyed) {
				created.free();
				return;
			}
			engine = created;
			phase = 'ready';
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
		unshaped = false;
		refused = false;
		if (!wanted || wanted === engineField) {
			engineRecipe = next;
			standing = false;
			here.command({ type: 'set_recipe', recipe });
			return;
		}
		if (!fieldPath) {
			// Ground this instance does not serve: said, not another planet shown.
			unshaped = true;
			return;
		}
		engineRecipe = next;
		standing = false;
		fetching = true;
		fetch(fieldPath)
			.then((response) => (response.ok ? response.arrayBuffer() : Promise.reject(response.status)))
			.then((bytes) => {
				if (here !== engine) return;
				here.set_field(new Uint8Array(bytes));
				engineField = wanted;
				fetching = false;
				here.command({ type: 'set_recipe', recipe });
			})
			.catch(() => {
				engineRecipe = '';
				fetching = false;
				phase = 'failed';
			});
	});

	// Each place the address bar is given is one request; `arrive` is read
	// outside the effect's tracking so a later world does not replay it.
	$effect(() => {
		const place = stand?.place;
		if (!place) return;
		pending = place;
		untrack(arrive);
	});

	$effect(() => {
		if (!engine || (avatar && avatar === engineAvatar)) return;
		if (avatar) engineAvatar = avatar;
		engine.command(avatar ? { type: 'set_avatar', path: avatar } : { type: 'random_avatar' });
	});

	$effect(() => {
		if (!engine || name === engineName) return;
		engineName = name;
		engine.command({ type: 'set_name', name });
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

{#snippet wait(word: string)}
	<div class="wait">
		<Spinner label={word} />
		<span aria-hidden="true">{word}</span>
	</div>
{/snippet}

<div class="view">
	<canvas bind:this={canvas} tabindex="0" aria-label={t('engine.canvas')} onclick={() => canvas?.focus()}></canvas>
	<div class="veil" class:lifted={status === 'running'}>
		{#if status === 'loading'}
			{@render wait(t('engine.loading'))}
		{:else if status === 'shaping'}
			{@render wait(t('engine.shaping'))}
		{:else if status === 'entering'}
			{@render wait(t('engine.entering'))}
		{:else if status === 'unshaped'}
			<Alert variant="danger" title={t('engine.unshaped.title')}>{t('engine.unshaped.body')}</Alert>
		{:else if status === 'refused'}
			<Alert variant="danger" title={t('engine.refused.title')}>{t('engine.refused.body')}</Alert>
		{:else if status === 'missing'}
			<Alert title={t('engine.missing.title')}>{t('engine.missing.body')} <code>bun run wasm</code></Alert>
		{:else if status === 'unsupported'}
			<Alert variant="danger" title={t('engine.unsupported.title')}>{t('engine.unsupported.body')}</Alert>
		{:else if status === 'failed'}
			<Alert variant="danger" title={t('engine.failed.title')}>{t('engine.failed.body')}</Alert>
		{/if}
	</div>
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
	/* Over the picture until it is the world asked for: softened, not hidden,
	   so the wait reads as a world coming and not as a blank. */
	.veil {
		position: absolute;
		inset: 0;
		display: grid;
		place-items: center;
		padding: var(--sp-5);
		pointer-events: none;
		background: color-mix(in srgb, var(--bg) 45%, transparent);
		backdrop-filter: blur(18px);
		-webkit-backdrop-filter: blur(18px);
		transition: opacity 300ms ease;
	}
	.veil.lifted {
		opacity: 0;
		visibility: hidden;
		transition:
			opacity 300ms ease,
			visibility 0s linear 300ms;
	}
	.wait {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--sp-3);
		color: var(--text-muted);
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
	}
</style>
