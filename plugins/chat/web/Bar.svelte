<script lang="ts">
	import { Icon, Input } from '$lib/ds';
	import { t } from './words';
	import { LINE_CHARS, type Line, type Scope } from './protocol';

	// A bar at the foot of the world. Closed, it shows what was just said and
	// fades it; Enter opens it, takes the pointer out of the world and puts
	// the caret in it. Enter again says the line and hands the world back.
	// The `@here` word is the bar's own, in the reader's language, and leaves
	// as a flag; the place comes back from the server, never from what was
	// typed.
	interface Props {
		lines: Line[];
		online: boolean;
		onsay: (scope: Scope, text: string, here: boolean) => void;
		ongo: (place: string) => void;
		/** Chat is opening: let go of the world. */
		onopen: () => void;
		/** Chat closed: take the world back. */
		onclose: () => void;
	}

	let { lines, online, onsay, ongo, onopen, onclose }: Props = $props();

	/** How long a line stays on a closed bar. */
	const RECENT_MS = 10_000;

	let scope = $state<Scope>('near');
	let draft = $state('');
	let open = $state(false);
	let input: HTMLInputElement | undefined = $state();
	let log: HTMLElement | undefined = $state();
	let now = $state(Date.now());

	const here = $derived(t('here'));
	// The server's limit, held here so nobody meets it there. Counted as the
	// server counts, in code points, so an accent is one. A draft is cut at
	// the limit as it is typed or pasted, so the line is never too long;
	// what is left is shown while the bar is open and stands out near zero.
	const left = $derived(LINE_CHARS - [...draft].length);
	const scarce = $derived(left <= LINE_CHARS / 10);
	$effect(() => {
		const chars = [...draft];
		if (chars.length > LINE_CHARS) draft = chars.slice(0, LINE_CHARS).join('');
	});
	const shown = $derived(open ? lines : lines.filter((line) => now - line.at < RECENT_MS));

	// A closed bar forgets: the clock ticks while there is something to forget.
	$effect(() => {
		if (open || lines.length === 0) return;
		const tick = setInterval(() => (now = Date.now()), 1000);
		return () => clearInterval(tick);
	});

	// The newest line is the one to read.
	$effect(() => {
		void shown.length;
		log?.scrollTo({ top: log.scrollHeight });
	});

	/** Whether a word of the draft is the share command, in this language or in English. */
	function shares(word: string): boolean {
		const lower = word.toLowerCase();
		return lower === here.toLowerCase() || lower === '@here';
	}

	function send() {
		const words = draft.split(/\s+/).filter(Boolean);
		const said = words.some(shares);
		const text = words.filter((word) => !shares(word)).join(' ');
		if (!text && !said) return;
		onsay(scope, text, said);
		draft = '';
	}

	function show() {
		if (!online || open) return;
		open = true;
		onopen();
		// The input is disabled until it is focused, so it lands after the flip.
		requestAnimationFrame(() => input?.focus());
	}

	function hide(back: boolean) {
		open = false;
		input?.blur();
		if (back) onclose();
	}

	function typing(event: KeyboardEvent): boolean {
		const target = event.target as HTMLElement | null;
		return !!target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable);
	}

	function onkey(event: KeyboardEvent) {
		if (event.key !== 'Enter' || typing(event) || event.repeat) return;
		event.preventDefault();
		show();
	}

	function onkeyInput(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault();
			send();
			hide(true);
		} else if (event.key === 'Escape') {
			event.preventDefault();
			hide(false);
		}
	}
</script>

<svelte:window onkeydown={onkey} />

<div class="chat" class:open>
	<ol class="log" bind:this={log} aria-label={t('lines')}>
		{#each shown as line (line.id)}
			<li class:own={line.own}>
				<span class="who">{line.who}</span>
				{#if line.scope === 'world'}<Icon name="globe" label={t('world')} />{/if}
				{#if line.place}
					<button type="button" class="place" title={t('goto', { place: line.place })} onclick={() => ongo(line.place ?? '')}>
						<Icon name="map-pin" />{line.place}
					</button>
				{/if}
				{#if line.text}<span class="text">{line.text}</span>{/if}
			</li>
		{/each}
	</ol>
	<div class="bar" role="group" aria-label={t('title')}>
		<div class="scopes" role="group" aria-label={t('scope')}>
			<button type="button" class="toggle" aria-pressed={scope === 'near'} title={t('near')} onclick={() => (scope = 'near')}>
				<Icon name="users" label={t('near')} />
			</button>
			<button type="button" class="toggle" aria-pressed={scope === 'world'} title={t('world')} onclick={() => (scope = 'world')}>
				<Icon name="globe" label={t('world')} />
			</button>
		</div>
		<Input
			bind:element={input}
			bind:value={draft}
			placeholder={!online ? t('offline') : open ? t('placeholder', { here }) : t('closed')}
			disabled={!online}
			enterkeyhint="send"
			autocomplete="off"
			aria-label={t('title')}
			onfocus={show}
			onkeydown={onkeyInput}
		/>
		{#if open}
			<span class="count" class:scarce title={t('left', { n: left })}>{left}</span>
		{/if}
		<button type="button" class="toggle" disabled={!online} title={t('send')} onclick={() => { send(); input?.focus(); }}>
			<Icon name="send" label={t('send')} />
		</button>
	</div>
</div>

<style>
	.chat {
		position: absolute;
		left: 50%;
		bottom: var(--sp-5);
		z-index: var(--z-panel);
		display: flex;
		flex-direction: column;
		gap: var(--sp-2);
		width: min(520px, calc(100% - 2 * var(--sp-4)));
		transform: translateX(-50%);
		pointer-events: none;
	}
	.log {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--sp-1);
		max-height: 30vh;
		padding-top: var(--sp-6);
		overflow: hidden;
		overflow-wrap: anywhere;
		/* The top edge fades, so a long log trails off instead of being cut. */
		mask-image: linear-gradient(to bottom, transparent, black var(--sp-7));
	}
	.open .log {
		overflow-y: auto;
		pointer-events: auto;
	}
	li {
		display: inline-flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: var(--sp-2);
		max-width: 100%;
		padding: var(--sp-1) var(--sp-3);
		background: var(--bg-overlay);
		backdrop-filter: blur(var(--sp-2));
		color: var(--text);
		pointer-events: auto;
		animation: arrive 200ms ease-out;
	}
	.who {
		color: var(--text-muted);
	}
	.own .who {
		color: var(--text);
		font-weight: var(--fw-bold);
	}
	.place {
		display: inline-flex;
		align-items: center;
		gap: var(--sp-1);
		padding: 0 var(--sp-2);
		border: var(--bw) solid var(--border-strong);
		border-radius: var(--radius);
		background: var(--bg);
		color: var(--text);
		font: inherit;
		cursor: pointer;
	}
	.place:hover {
		background: var(--action);
		color: var(--action-text);
	}
	.bar {
		display: flex;
		align-items: center;
		gap: var(--sp-2);
		padding: var(--sp-2);
		border: var(--bw) solid var(--border-strong);
		background: var(--bg-overlay);
		backdrop-filter: blur(var(--sp-2));
		pointer-events: auto;
	}
	.scopes {
		display: flex;
	}
	.count {
		flex: none;
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
	}
	.count.scarce {
		color: var(--text);
		font-weight: var(--fw-bold);
	}
	.toggle {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: var(--control-h);
		height: var(--control-h);
		border: var(--bw) solid var(--border);
		background: var(--bg);
		color: var(--text-muted);
		cursor: pointer;
	}
	.scopes .toggle + .toggle {
		margin-left: calc(-1 * var(--bw));
	}
	.toggle:hover {
		color: var(--text);
		border-color: var(--border-strong);
	}
	.toggle[aria-pressed='true'] {
		background: var(--action);
		color: var(--action-text);
		border-color: var(--border-strong);
	}
	.toggle:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	@keyframes arrive {
		from {
			opacity: 0;
			transform: translateY(var(--sp-2));
		}
	}
	@media (prefers-reduced-motion: reduce) {
		li {
			animation: none;
		}
	}
</style>
