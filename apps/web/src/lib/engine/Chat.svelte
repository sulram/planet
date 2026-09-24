<script module lang="ts">
	import type { Scope } from './index';

	/** One line as the panel shows it: who said it, with the name they had then. */
	export interface Line {
		id: number;
		who: string;
		own: boolean;
		scope: Scope;
		text: string;
		/** Where they stood when they shared it, as `go_to` takes it. */
		place: string | null;
	}
</script>

<script lang="ts">
	import { Button, Input, Segmented } from '$lib/ds';
	import { t } from '$lib/i18n';
	import { LINE_BYTES, scopes } from './index';

	// The chat as a front end: lines come in as events, a line goes out as a
	// command. The `@here` word is the panel's own, in the reader's language,
	// and leaves as a flag; the place comes back from the server, never from
	// what was typed.
	interface Props {
		lines: Line[];
		online: boolean;
		onsay: (scope: Scope, text: string, here: boolean) => void;
		ongo: (place: string) => void;
	}

	let { lines, online, onsay, ongo }: Props = $props();

	let scope = $state<Scope>('near');
	let draft = $state('');
	let log: HTMLElement | undefined = $state();

	const here = $derived(t('engine.chat.here'));
	const scopeOptions = $derived(scopes.map((value) => ({ value, label: t(`engine.chat.${value}`) })));
	// The server's limit, held here so nobody meets it there.
	const tooLong = $derived(new TextEncoder().encode(draft).length > LINE_BYTES);

	/** Whether a word of the draft is the share command, in this language or in English. */
	function shares(word: string): boolean {
		const lower = word.toLowerCase();
		return lower === here.toLowerCase() || lower === '@here';
	}

	function send(event: SubmitEvent) {
		event.preventDefault();
		const words = draft.split(/\s+/).filter(Boolean);
		const said = words.some(shares);
		const text = words.filter((word) => !shares(word)).join(' ');
		if ((!text && !said) || tooLong) return;
		onsay(scope, text, said);
		draft = '';
	}

	// The newest line is the one to read.
	$effect(() => {
		void lines.length;
		log?.scrollTo({ top: log.scrollHeight });
	});
</script>

<section class="chat">
	<header>
		<h3>{t('engine.chat')}</h3>
	</header>
	<ol class="log" bind:this={log} aria-label={t('engine.chat.lines')}>
		{#each lines as line (line.id)}
			<li class:own={line.own}>
				<span class="who">{line.who}</span>
				{#if line.scope === 'world'}<span class="scope">{t('engine.chat.world')}</span>{/if}
				{#if line.place}
					<button type="button" class="place" title={t('engine.chat.goto', { place: line.place })} onclick={() => ongo(line.place ?? '')}>
						{line.place}
					</button>
				{/if}
				{#if line.text}<span class="text">{line.text}</span>{/if}
			</li>
		{/each}
	</ol>
	<form onsubmit={send}>
		<Segmented options={scopeOptions} value={scope} label={t('engine.chat.scope')} onselect={(value) => (scope = value)} />
		<div class="row">
			<Input
				bind:value={draft}
				placeholder={online ? t('engine.chat.placeholder', { here }) : t('engine.chat.offline')}
				disabled={!online}
				invalid={tooLong}
				enterkeyhint="send"
				autocomplete="off"
				aria-label={t('engine.chat')}
			/>
			<Button type="submit" variant="ghost" disabled={!online || tooLong}>{t('engine.chat.send')}</Button>
		</div>
	</form>
</section>

<style>
	.chat {
		display: flex;
		flex-direction: column;
		gap: var(--sp-2);
	}
	h3 {
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
		color: var(--text-muted);
	}
	.log {
		display: flex;
		flex-direction: column;
		gap: var(--sp-1);
		max-height: 12rem;
		overflow-y: auto;
		overflow-wrap: anywhere;
	}
	.who {
		color: var(--text-muted);
	}
	.own .who {
		color: var(--text);
	}
	.scope {
		color: var(--text-muted);
	}
	.place {
		padding: 0 var(--sp-1);
		border: var(--bw) solid var(--border-strong);
		border-radius: var(--radius);
		background: var(--bg);
		color: var(--text);
		font: inherit;
		cursor: pointer;
	}
	.place:hover {
		border-color: var(--text);
	}
	form {
		display: flex;
		flex-direction: column;
		gap: var(--sp-2);
	}
	.row {
		display: flex;
		gap: var(--sp-2);
	}
</style>
