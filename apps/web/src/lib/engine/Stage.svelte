<script lang="ts">
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import { Badge, Button, Icon, IconButton, Input, Panel, Segmented, Stat, ThemeToggle } from '$lib/ds';
	import { t } from '$lib/i18n';
	import type { Recipe } from '$lib/world';
	import type { Link } from '$lib/server/session';
	import { onMount } from 'svelte';
	import { replaceState } from '$app/navigation';
	import Balloons from './Balloons.svelte';
	import Build from './Build.svelte';
	import Chat, { type Line } from './Chat.svelte';
	import EngineView from './EngineView.svelte';
	import Help from './Help.svelte';
	import Settings from './Settings.svelte';
	import SignIn from './SignIn.svelte';
	import {
		modes,
		NAME_CHARS,
		type Anchor,
		type BuildRefusal,
		type Effects,
		type EngineEvent,
		type Mode,
		type PeerInfo,
		type Scope,
		type SessionStatus,
		type Tool
	} from './index';
	import { who } from './who';

	// The full viewport engine with its floating panel: what `/play` and
	// `/w/[id]` share. The page may supply the top of the panel; who is here,
	// the mode and the stats line are the same everywhere. The keys live
	// behind the help button and the picture behind settings.
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
		/** What this person is called, from the page load. Empty when nobody said. */
		name?: string;
		/** The world's socket, when this is a world other people can be in. */
		link?: Link;
		/** Called once the engine reports which generator version it runs. */
		onready?: (generatorVersion: number) => void;
		/** The page's own top of the panel, when it has one. */
		children?: Snippet;
	}

	let { title, recipe, fieldPath, avatar, name = '', link, onready, children }: Props = $props();

	// The name, edited where it is shown. The engine hears it at once, so the
	// world does too, and the page keeps it for the next visit: on the account
	// when signed in, in a cookie for a visitor. Best effort, like the avatar.
	// The page's name is the starting point only: from here on it is edited.
	// svelte-ignore state_referenced_locally
	let myName = $state(name);
	let editing = $state(false);
	let draft = $state('');
	let nameInput: HTMLInputElement | undefined = $state();

	function editName() {
		draft = myName;
		editing = true;
		requestAnimationFrame(() => nameInput?.select());
	}

	function keepName() {
		if (!editing) return;
		editing = false;
		const next = [...draft.split(/\s+/).filter(Boolean).join(' ')].slice(0, NAME_CHARS).join('').trim();
		if (next === myName) return;
		myName = next;
		fetch('/name', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ name: next })
		}).catch(() => {});
	}

	function onNameKey(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault();
			keepName();
		} else if (event.key === 'Escape') {
			event.preventDefault();
			editing = false;
		}
	}

	// Signing in happens here, over the world (DECISIONS 70). Once the
	// session is set the page reloads: the socket then opens with a ticket
	// and the name is the account's, and the address bar keeps the place.
	let signingIn = $state(false);
	const user = $derived(page.data.user);
	$effect(() => {
		if (signingIn) view?.release();
		else view?.take();
	});

	let view = $state<ReturnType<typeof EngineView>>();
	let mode = $state<Mode>('walk');
	let stats = $state<Extract<EngineEvent, { type: 'stats' }>>();
	let session = $state<SessionStatus>('offline');
	/** This client's own session in the world, while online. */
	let me = $state<number | null>(null);
	let peers = $state<PeerInfo[]>([]);
	let anchors = $state.raw<Anchor[]>([]);

	// Building: what the engine says is in hand, and why it last found no room.
	let tool = $state<Tool | null>(null);
	let paint = $state(0);
	let palette = $state.raw<string[]>([]);
	let refused = $state<BuildRefusal | null>(null);
	let history = $state({ undo: false, redo: false });

	function build(next: Tool | null) {
		refused = null;
		view?.command({ type: 'set_tool', tool: next });
	}

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

	// The address bar is where you are. What this page writes there as you
	// move is where you already stand; what a hand puts there is somewhere to
	// go: the link that brought you here, a place pasted over it, a world
	// entered from another. `page.url` moves on those alone, since the page's
	// own `replaceState` leaves it be, and a new object each time makes the
	// same place pasted twice two requests. The engine view walks there once
	// it stands in the world this page asked for (DECISIONS 73). Empty on the
	// server, where no address bar has a hash.
	/** The last pose this page wrote, so standing still writes nothing. */
	const own = { pose: '' };
	const arrival = $derived.by(() => {
		const place = page.url.hash.slice(1);
		return place && place !== own.pose ? { place } : null;
	});

	// Replaced and never pushed: the back button is the way out of the world,
	// not a trail of every step taken in it.
	$effect(() => {
		if (!stats || stats.pose === own.pose) return;
		own.pose = stats.pose;
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
		else if (event.type === 'palette') palette = event.colors;
		else if (event.type === 'tool_changed') {
			tool = event.tool;
			paint = event.paint;
			if (tool) refused = null;
		} else if (event.type === 'build_refused') refused = event.reason;
		else if (event.type === 'history') history = { undo: event.undo, redo: event.redo };
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
	<EngineView bind:this={view} {recipe} {fieldPath} {mode} {avatar} name={myName} {link} stand={arrival} effects={wanted} onevent={receive} />
	{#if link}
		<Balloons {anchors} {peers} {me} {lines} />
		<Chat {lines} online={session === 'online'} onsay={say} ongo={go} onopen={() => view?.release()} onclose={() => view?.take()} />
	{/if}
	<Settings {effects} {defaults} onchange={choose} />
	<Build
		{tool}
		{paint}
		{palette}
		{refused}
		{history}
		onbuild={build}
		onpaint={(next) => view?.command({ type: 'set_paint', paint: next })}
		onundo={() => view?.command({ type: 'undo' })}
		onredo={() => view?.command({ type: 'redo' })}
	/>
	<Help />
	<SignIn bind:open={signingIn} ondone={() => location.reload()} />
	<Panel {title}>
		{#snippet aside()}
			<span class="corner">
				{#if user?.operator}<IconButton href="/backoffice" icon="layout-dashboard" label={t('nav.backoffice')} />{/if}
				{#if user}
					<form method="POST" action="/logout">
						<IconButton type="submit" icon="log-out" label={t('auth.logout')} />
					</form>
				{:else}
					<Button variant="ghost" onclick={() => (signingIn = true)}>{t('auth.login.title')}</Button>
				{/if}
				<ThemeToggle />
			</span>
		{/snippet}
		{@render children?.()}
		{#if link}
			<section class="here">
				<header>
					<h3>{t('engine.here')}</h3>
					<Badge variant={session === 'online' ? 'solid' : 'outline'}>{t(`engine.session.${session}`)}</Badge>
				</header>
				<ul>
					{#if session === 'online'}
						<li class="you">
							{#if editing}
								<Input
									bind:element={nameInput}
									bind:value={draft}
									maxlength={NAME_CHARS * 2}
									placeholder={t('engine.here.name')}
									aria-label={t('engine.here.name')}
									autocomplete="off"
									spellcheck="false"
									onkeydown={onNameKey}
									onblur={keepName}
								/>
							{:else}
								<span>{myName || t('engine.here.you')}</span>
								{#if myName}<span class="muted">{t('engine.here.you')}</span>{/if}
								<button type="button" class="edit" title={t('engine.here.rename')} onclick={editName}>
									<Icon name="pencil" label={t('engine.here.rename')} />
								</button>
							{/if}
						</li>
					{/if}
					{#each peers as peer (peer.session)}
						<li>{who(peer)}</li>
					{/each}
				</ul>
			</section>
		{/if}
		<Segmented options={modeOptions} value={mode} label={t('engine.mode')} onselect={(value) => (mode = value)} />
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
	.corner {
		display: flex;
		align-items: center;
		gap: var(--sp-2);
	}
	/* the form around sign out would otherwise sit on the text baseline */
	.corner form {
		display: flex;
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
		display: flex;
		align-items: center;
		gap: var(--sp-2);
		min-height: var(--control-h);
	}
	.muted {
		color: var(--text-muted);
	}
	.edit {
		display: inline-grid;
		place-items: center;
		width: var(--control-h);
		height: var(--control-h);
		border: none;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
	}
	.edit:hover {
		color: var(--text);
	}
</style>
