<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Panel, Segmented, Stat } from '$lib/ds';
	import { t } from '$lib/i18n';
	import type { Recipe } from '$lib/world';
	import { onMount } from 'svelte';
	import { replaceState } from '$app/navigation';
	import EngineView from './EngineView.svelte';
	import Settings from './Settings.svelte';
	import { modes, type Effects, type EngineEvent, type Mode } from './index';

	// The full viewport engine with its floating panel: what `/play` and
	// `/w/[id]` share. The page supplies the top of the panel; mode, key hints
	// and the stats line are the same everywhere.
	// The compass point a bearing lands on, and the angle beside it. A rose has
	// sixteen points, so each is 22.5 degrees wide. `null` is a pole, where a
	// bearing is not a thing that exists.
	function facing(deg: number | null): string {
		if (deg === null) return t('engine.stats.atThePole');
		const rose = t('engine.compass.rose').split(',');
		const point = rose[Math.round(deg / 22.5) % rose.length];
		return `${point} ${t('engine.stats.degrees', { n: deg.toFixed(0) })}`;
	}

	interface Props {
		title: string;
		recipe: Recipe | null;
		/** Where to fetch the field the recipe names, when it names one. */
		fieldPath?: string;
		/** Asset path of the visitor's avatar, from the page load. */
		avatar: string | null;
		/** Called once the engine reports which generator version it runs. */
		onready?: (generatorVersion: number) => void;
		children: Snippet;
	}

	let { title, recipe, fieldPath, avatar, onready, children }: Props = $props();

	let mode = $state<Mode>('walk');
	let stats = $state<Extract<EngineEvent, { type: 'stats' }>>();

	// The address bar is where you are. Read once, synchronously, because the
	// engine asks for it as soon as its first world is built; `null` on the
	// server, where there is no address bar to read.
	const arrivedAt = typeof location === 'undefined' ? null : location.hash.slice(1) || null;
	/** Where a respawn puts you back: the address on arrival, then wherever you are. */
	const stand = $derived(stats?.place ?? arrivedAt);
	/** The last place written, so standing still writes nothing. */
	let written = '';

	// Replaced and never pushed: the back button is the way out of the world,
	// not a trail of every step taken in it.
	$effect(() => {
		if (!stats || stats.place === written) return;
		written = stats.place;
		replaceState(`#${stats.place}`, {});
	});

	// How the picture is made belongs to the machine, not to the account: a
	// phone and a desktop want different answers. It stays in this browser.
	const EFFECTS_KEY = 'planet.effects';
	/** What is asked of the engine: the stored choice, then the panel. */
	let wanted = $state<Effects>();
	/** What the engine says is in force, and what it started with. */
	let effects = $state<Effects>();
	let defaults = $state<Effects>();

	onMount(() => {
		try {
			const stored = localStorage.getItem(EFFECTS_KEY);
			if (stored) wanted = JSON.parse(stored);
		} catch {
			// unreadable or blocked storage: the engine's defaults stand
		}
	});

	function choose(next: Effects) {
		wanted = next;
		try {
			localStorage.setItem(EFFECTS_KEY, JSON.stringify(next));
		} catch {
			// best effort: the choice then lasts for this visit
		}
	}

	const modeOptions = $derived(modes.map((value) => ({ value, label: t(`engine.mode.${value}`) })));
	const hints = $derived([
		{ keys: t('engine.hint.look.keys'), does: t('engine.hint.look') },
		{ keys: 'W A S D', does: t('engine.hint.move') },
		{ keys: t('engine.hint.up.keys'), does: t('engine.hint.up') },
		{ keys: 'C', does: t('engine.hint.down') },
		{ keys: 'Shift', does: t('engine.hint.sprint') },
		{ keys: 'F', does: t('engine.hint.mode') },
		{ keys: 'V', does: t('engine.hint.avatar') },
		{ keys: t('engine.hint.zoom.keys'), does: t('engine.hint.zoom') },
		{ keys: 'Esc', does: t('engine.hint.release') }
	]);

	function receive(event: EngineEvent) {
		if (event.type === 'ready') onready?.(event.generator_version);
		else if (event.type === 'mode_changed') mode = event.mode;
		else if (event.type === 'stats') stats = event;
		else if (event.type === 'effects_changed') {
			defaults ??= event.effects;
			effects = event.effects;
		}
		else if (event.type === 'avatar_changed' && event.path !== avatar) remember(event.path);
	}

	// An avatar picked in the world becomes the visitor's choice. Best effort:
	// a failure only means the next visit starts from the previous choice.
	function remember(path: string) {
		fetch('/avatar', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ path })
		}).catch(() => {});
	}
</script>

<div class="stage">
	<EngineView {recipe} {fieldPath} {mode} {avatar} {stand} effects={wanted} onevent={receive} />
	<Settings {effects} {defaults} onchange={choose} />
	<Panel {title}>
		{#snippet aside()}
			<a class="home" href="/">{t('common.appName')}</a>
		{/snippet}
		{@render children()}
		<Segmented options={modeOptions} value={mode} label={t('engine.mode')} onselect={(value) => (mode = value)} />
		<ul class="hints">
			{#each hints as hint (hint.keys)}
				<li><kbd>{hint.keys}</kbd> {hint.does}</li>
			{/each}
		</ul>
		{#if stats}
			<dl>
				<Stat label={t('engine.stats.place')} value={stats.place} />
				<Stat label={t('engine.stats.facing')} value={facing(stats.bearing_deg)} />
				<Stat label={t('engine.stats.altitude')} value={t('engine.stats.metres', { n: stats.altitude_m.toFixed(1) })} />
				<Stat label={t('engine.stats.speed')} value={t('engine.stats.metresPerSecond', { n: stats.speed_mps.toFixed(1) })} />
				<Stat label={t('engine.stats.fps')} value={stats.fps.toFixed(0)} />
			</dl>
		{/if}
	</Panel>
</div>

<style>
	.stage {
		position: fixed;
		inset: 0;
		overflow: hidden;
	}
	.home {
		color: var(--text-muted);
		text-decoration: none;
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
	}
	.home:hover {
		color: var(--text);
	}
	.hints {
		display: flex;
		flex-direction: column;
		gap: var(--sp-1);
		color: var(--text-muted);
	}
	kbd {
		color: var(--text);
	}
</style>
