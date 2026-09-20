<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Panel, Segmented, Stat } from '$lib/ds';
	import { t } from '$lib/i18n';
	import type { Recipe } from '$lib/world';
	import EngineView from './EngineView.svelte';
	import { modes, type EngineEvent, type Mode } from './index';

	// The full viewport engine with its floating panel: what `/play` and
	// `/w/[id]` share. The page supplies the top of the panel; mode, key hints
	// and the stats line are the same everywhere.
	interface Props {
		title: string;
		recipe: Recipe | null;
		/** Called once the engine reports which generator version it runs. */
		onready?: (generatorVersion: number) => void;
		children: Snippet;
	}

	let { title, recipe, onready, children }: Props = $props();

	let mode = $state<Mode>('walk');
	let stats = $state<Extract<EngineEvent, { type: 'stats' }>>();

	const modeOptions = $derived(modes.map((value) => ({ value, label: t(`engine.mode.${value}`) })));
	const hints = $derived([
		{ keys: t('engine.hint.look.keys'), does: t('engine.hint.look') },
		{ keys: 'W A S D', does: t('engine.hint.move') },
		{ keys: t('engine.hint.up.keys'), does: t('engine.hint.up') },
		{ keys: 'C', does: t('engine.hint.down') },
		{ keys: 'Shift', does: t('engine.hint.sprint') },
		{ keys: 'F', does: t('engine.hint.mode') },
		{ keys: t('engine.hint.zoom.keys'), does: t('engine.hint.zoom') },
		{ keys: 'Esc', does: t('engine.hint.release') }
	]);

	function receive(event: EngineEvent) {
		if (event.type === 'ready') onready?.(event.generator_version);
		else if (event.type === 'mode_changed') mode = event.mode;
		else if (event.type === 'stats') stats = event;
	}
</script>

<div class="stage">
	<EngineView {recipe} {mode} onevent={receive} />
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
				<Stat label={t('engine.stats.fps')} value={stats.fps.toFixed(0)} />
				<Stat label={t('engine.stats.altitude')} value={t('engine.stats.metres', { n: stats.altitude_m.toFixed(1) })} />
				<Stat label={t('engine.stats.speed')} value={t('engine.stats.metresPerSecond', { n: stats.speed_mps.toFixed(1) })} />
				<Stat label={t('engine.stats.sector')} value={String(stats.sector)} />
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
