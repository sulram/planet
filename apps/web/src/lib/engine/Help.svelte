<script lang="ts">
	import { Button, Icon, IconButton, Panel, type IconName } from '$lib/ds';
	import { t } from '$lib/i18n';
	import type { Hint } from '$lib/plugins/plugin';

	// The keys, behind a button in the bottom left corner: read once, then
	// out of the way of the world. The core's own, and between them the keys
	// of each plugin that is on in this world. On a touch screen it says
	// what the fingers and the buttons do instead, as a panel of the bar,
	// which opens and closes it: the bottom left is the stick's.
	interface Props {
		plugins?: readonly Hint[];
		/** `V` browses the avatars on offer, unless the account wears its own. */
		browse?: boolean;
		/** A touch screen: the panel alone, under the bar. */
		touch?: boolean;
		onclose?: () => void;
	}

	let { plugins = [], browse = true, touch = false, onclose }: Props = $props();
	let open = $state(false);

	type Line = { keys?: string; icon?: IconName; does: string };

	const keys = $derived<Line[]>([
		{ keys: t('engine.hint.look.keys'), does: t('engine.hint.look') },
		{ keys: 'W A S D', does: t('engine.hint.move') },
		{ keys: t('engine.hint.up.keys'), does: t('engine.hint.up') },
		{ keys: 'C', does: t('engine.hint.down') },
		{ keys: 'Shift', does: t('engine.hint.sprint') },
		{ keys: 'F', does: t('engine.hint.mode') },
		...(browse ? [{ keys: 'V', does: t('engine.hint.avatar') }] : []),
		{ keys: 'P', does: t('engine.hint.eye') },
		{ keys: t('engine.hint.undo.keys'), does: t('engine.hint.undo') },
		...plugins.map((hint) => ({ keys: typeof hint.keys === 'string' ? hint.keys : hint.keys(), does: hint.does() })),
		{ keys: t('engine.hint.zoom.keys'), does: t('engine.hint.zoom') },
		{ keys: 'Esc', does: t('engine.hint.release') }
	]);

	// A plugin's hints are keys, which a touch screen has none of.
	const fingers = $derived<Line[]>([
		{ keys: t('engine.hint.move.touch'), does: t('engine.hint.move') },
		{ keys: t('engine.hint.sprint.touch'), does: t('engine.hint.sprint') },
		{ keys: t('engine.hint.look.touch'), does: t('engine.hint.look') },
		{ keys: t('engine.hint.zoom.touch'), does: t('engine.hint.zoom') },
		{ icon: 'chevrons-up', does: t('engine.hint.up') },
		{ icon: 'chevrons-down', does: t('engine.hint.down') },
		{ icon: 'plane', does: t('engine.hint.mode') },
		{ icon: 'eye', does: t('engine.hint.eye') }
	]);
</script>

{#snippet hints(lines: Line[])}
	<ul class="hints">
		{#each lines as line (line.does)}
			<li><kbd>{#if line.icon}<Icon name={line.icon} />{:else}{line.keys}{/if}</kbd> {line.does}</li>
		{/each}
	</ul>
{/snippet}

{#if touch}
	<Panel title={t('engine.help')} corner="top-right" under>
		{#snippet aside()}
			<IconButton icon="x" label={t('common.close')} onclick={onclose} />
		{/snippet}
		{@render hints(fingers)}
	</Panel>
{:else if open}
	<Panel title={t('engine.help')} corner="bottom-left">
		{#snippet aside()}
			<button class="close" type="button" onclick={() => (open = false)}>{t('common.close')}</button>
		{/snippet}
		{@render hints(keys)}
	</Panel>
{:else}
	<div class="toggle">
		<Button variant="ghost" type="button" aria-expanded="false" onclick={() => (open = true)}>{t('engine.help')}</Button>
	</div>
{/if}

<style>
	.toggle {
		position: absolute;
		bottom: var(--sp-5);
		left: var(--sp-4);
		z-index: var(--z-panel);
		background: var(--bg-overlay);
	}
	.close {
		border: none;
		background: none;
		color: var(--text-muted);
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
		cursor: pointer;
	}
	.close:hover {
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
