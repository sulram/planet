<script lang="ts">
	import { Button, Checkbox, Panel, Slider, Stack } from '$lib/ds';
	import { t } from '$lib/i18n';
	import type { Effects } from './index';

	// How the picture is made: a button in the top right corner, and the panel
	// it opens. A front-end only: it shows what the engine said is in force and
	// asks for changes; the engine clamps and answers.
	interface Props {
		/** What the engine last reported. Undefined until it has. */
		effects: Effects | undefined;
		/** What the engine starts with, to offer a way back. */
		defaults: Effects | undefined;
		onchange: (effects: Effects) => void;
	}

	let { effects, defaults, onchange }: Props = $props();

	let open = $state(false);

	function set<K extends keyof Effects>(key: K, value: Effects[K]) {
		if (effects) onchange({ ...effects, [key]: value });
	}
</script>

{#if effects}
	{#if open}
		<Panel title={t('engine.settings')} corner="top-right">
			{#snippet aside()}
				<button class="close" type="button" onclick={() => (open = false)}>{t('common.close')}</button>
			{/snippet}
			<Stack>
				<Checkbox checked={effects.shadows} onchange={(value) => set('shadows', value)}>{t('engine.settings.shadows')}</Checkbox>
				<Checkbox checked={effects.grass} onchange={(value) => set('grass', value)}>{t('engine.settings.grass')}</Checkbox>
				<Checkbox checked={effects.clouds} onchange={(value) => set('clouds', value)}>{t('engine.settings.clouds')}</Checkbox>
			</Stack>
			<Stack>
				<Slider label={t('engine.settings.cloudCover')} value={effects.cloud_cover} min={0} max={1} disabled={!effects.clouds} oninput={(value) => set('cloud_cover', value)} />
				<Slider label={t('engine.settings.cloudDensity')} value={effects.cloud_density} min={0.1} max={3} disabled={!effects.clouds} oninput={(value) => set('cloud_density', value)} />
				<Slider
					label={t('engine.settings.wind')}
					value={effects.wind_m_s}
					min={0}
					max={80}
					step={1}
					format={(n) => t('engine.stats.metresPerSecond', { n: n.toFixed(0) })}
					disabled={!effects.clouds}
					oninput={(value) => set('wind_m_s', value)}
				/>
				<Slider label={t('engine.settings.cloudChange')} value={effects.cloud_change} min={0} max={6} step={0.1} disabled={!effects.clouds} oninput={(value) => set('cloud_change', value)} />
			</Stack>
			<Slider label={t('engine.settings.exposure')} value={effects.exposure} min={0.1} max={4} oninput={(value) => set('exposure', value)} />
			{#if defaults}
				<Button variant="ghost" type="button" onclick={() => defaults && onchange(defaults)}>{t('engine.settings.defaults')}</Button>
			{/if}
		</Panel>
	{:else}
		<div class="toggle">
			<Button variant="ghost" type="button" aria-expanded="false" onclick={() => (open = true)}>{t('engine.settings')}</Button>
		</div>
	{/if}
{/if}

<style>
	.toggle {
		position: absolute;
		top: var(--sp-4);
		right: var(--sp-4);
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
</style>
