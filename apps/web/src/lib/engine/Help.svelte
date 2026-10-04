<script lang="ts">
	import { Button, Panel } from '$lib/ds';
	import { t } from '$lib/i18n';
	import type { Hint } from '$lib/plugins/plugin';

	// The keys, behind a button in the bottom left corner: read once, then
	// out of the way of the world. The core's own, and between them the keys
	// of each plugin that is on in this world.
	let { plugins = [] }: { plugins?: readonly Hint[] } = $props();
	let open = $state(false);

	const hints = $derived([
		{ keys: t('engine.hint.look.keys'), does: t('engine.hint.look') },
		{ keys: 'W A S D', does: t('engine.hint.move') },
		{ keys: t('engine.hint.up.keys'), does: t('engine.hint.up') },
		{ keys: 'C', does: t('engine.hint.down') },
		{ keys: 'Shift', does: t('engine.hint.sprint') },
		{ keys: 'F', does: t('engine.hint.mode') },
		{ keys: 'V', does: t('engine.hint.avatar') },
		...plugins.map((hint) => ({ keys: typeof hint.keys === 'string' ? hint.keys : hint.keys(), does: hint.does() })),
		{ keys: t('engine.hint.zoom.keys'), does: t('engine.hint.zoom') },
		{ keys: 'Esc', does: t('engine.hint.release') }
	]);
</script>

{#if open}
	<Panel title={t('engine.help')} corner="bottom-left">
		{#snippet aside()}
			<button class="close" type="button" onclick={() => (open = false)}>{t('common.close')}</button>
		{/snippet}
		<ul class="hints">
			{#each hints as hint (hint.does)}
				<li><kbd>{hint.keys}</kbd> {hint.does}</li>
			{/each}
		</ul>
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
