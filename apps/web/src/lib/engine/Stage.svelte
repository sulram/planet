<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Badge, Panel, Segmented, Stat } from '$lib/ds';
	import { t } from '$lib/i18n';
	import type { Recipe } from '$lib/world';
	import type { Link } from '$lib/server/session';
	import { onMount } from 'svelte';
	import { replaceState } from '$app/navigation';
	import Balloons from './Balloons.svelte';
	import Chat, { type Line } from './Chat.svelte';
	import EngineView from './EngineView.svelte';
	import Settings from './Settings.svelte';
	import { modes, type Anchor, type Effects, type EngineEvent, type Mode, type PeerInfo, type Scope, type SessionStatus } from './index';
	import { who } from './who';

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
		/** The world's socket, when this is a world other people can be in. */
		link?: Link;
		/** Called once the engine reports which generator version it runs. */
		onready?: (generatorVersion: number) => void;
		children: Snippet;
	}

	let { title, recipe, fieldPath, avatar, link, onready, children }: Props = $props();

	let view = $state<ReturnType<typeof EngineView>>();
	let mode = $state<Mode>('walk');
	let stats = $state<Extract<EngineEvent, { type: 'stats' }>>();
	let session = $state<SessionStatus>('offline');
	/** This client's own session in the world, while online. */
	let me = $state<number | null>(null);
	let peers = $state<PeerInfo[]>([]);
	let anchors = $state.raw<Anchor[]>([]);

	// Lines are what was heard while here, never stored: the panel keeps the
	// last hundred and a line keeps the name its speaker had when it was said.
	const LINES_KEPT = 100;
	let lines = $state<Line[]>([]);
	let lineCount = 0;

	function heard(event: Extract<EngineEvent, { type: 'said' }>) {
		const own = event.session === me;
		const peer = peers.find((p) => p.session === event.session);
		const line: Line = {
			id: ++lineCount,
			session: event.session,
			who: own ? t('engine.here.you') : who(peer ?? { session: event.session, name: '', visitor: false }),
			own,
			scope: event.scope,
			text: event.text,
			place: event.place,
			at: Date.now()
		};
		lines = [...lines.slice(1 - LINES_KEPT), line];
	}

	function say(scope: Scope, text: string, here: boolean) {
		view?.command({ type: 'say', scope, text, here });
	}

	function go(place: string) {
		view?.command({ type: 'go_to', place });
	}

	// The address bar is where you are. Read once, synchronously, because the
	// engine asks for it as soon as its first world is built; `null` on the
	// server, where there is no address bar to read.
	const arrivedAt = typeof location === 'undefined' ? null : location.hash.slice(1) || null;
	/** The last place written, so standing still writes nothing. */
	let written = '';

	// Replaced and never pushed: the back button is the way out of the world,
	// not a trail of every step taken in it.
	$effect(() => {
		if (!stats || stats.pose === written) return;
		written = stats.pose;
		replaceState(`#${stats.pose}`, {});
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
		{ keys: 'Enter', does: t('engine.hint.chat') },
		{ keys: t('engine.hint.zoom.keys'), does: t('engine.hint.zoom') },
		{ keys: 'Esc', does: t('engine.hint.release') }
	]);

	function receive(event: EngineEvent) {
		if (event.type === 'ready') onready?.(event.generator_version);
		else if (event.type === 'mode_changed') mode = event.mode;
		else if (event.type === 'stats') stats = event;
		else if (event.type === 'session') {
			session = event.status;
			me = event.session;
		} else if (event.type === 'peers') peers = event.peers;
		else if (event.type === 'said') heard(event);
		else if (event.type === 'anchors') anchors = event.anchors;
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
	<EngineView bind:this={view} {recipe} {fieldPath} {mode} {avatar} {link} stand={arrivedAt} effects={wanted} onevent={receive} />
	{#if link}
		<Balloons {anchors} {peers} {me} {lines} />
		<Chat {lines} online={session === 'online'} onsay={say} ongo={go} onopen={() => view?.release()} onclose={() => view?.take()} />
	{/if}
	<Settings {effects} {defaults} onchange={choose} />
	<Panel {title}>
		{#snippet aside()}
			<a class="home" href="/">{t('common.appName')}</a>
		{/snippet}
		{@render children()}
		{#if link}
			<section class="here">
				<header>
					<h3>{t('engine.here')}</h3>
					<Badge variant={session === 'online' ? 'solid' : 'outline'}>{t(`engine.session.${session}`)}</Badge>
				</header>
				<ul>
					{#if session === 'online'}
						<li class="you">{t('engine.here.you')}</li>
					{/if}
					{#each peers as peer (peer.session)}
						<li>{who(peer)}</li>
					{/each}
				</ul>
			</section>
		{/if}
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
	.here {
		display: flex;
		flex-direction: column;
		gap: var(--sp-2);
	}
	.here header {
		display: flex;
		justify-content: space-between;
		gap: var(--sp-3);
	}
	.here h3 {
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
		color: var(--text-muted);
	}
	.here ul {
		display: flex;
		flex-direction: column;
		gap: var(--sp-1);
	}
	.you {
		color: var(--text-muted);
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
