<script lang="ts">
	import { Button, Checkbox, IconButton, Panel, Segmented, Select, Slider, Stack, Stat } from '$lib/ds';
	import { t } from '$lib/i18n';
	import { modes, toneMaps, type Effects, type Mode } from './index';

	// Settings: how the body moves and how the picture is made, in the panel
	// the gear in the top right corner opens. A front end only: it shows what
	// the engine said is in force and asks for changes; the engine clamps and
	// answers.
	interface Props {
		mode: Mode;
		onmode: (mode: Mode) => void;
		/** What the engine last reported. Undefined until it has. */
		effects: Effects | undefined;
		/** What the engine starts with, to offer a way back. */
		defaults: Effects | undefined;
		onchange: (effects: Effects) => void;
		/** Frames a second, as the engine last counted them. */
		fps: number | undefined;
		onclose: () => void;
	}

	let { mode, onmode, effects, defaults, onchange, fps, onclose }: Props = $props();

	const modeOptions = $derived(modes.map((value) => ({ value, label: t(`engine.mode.${value}`) })));
	const toneMapOptions = $derived(toneMaps.map((value) => ({ value, label: t(`engine.settings.toneMap.${value}`) })));

	function set<K extends keyof Effects>(key: K, value: Effects[K]) {
		if (effects) onchange({ ...effects, [key]: value });
	}
</script>

<Panel title={t('engine.settings')} corner="top-right" under>
	{#snippet aside()}
		<IconButton icon="x" label={t('common.close')} onclick={onclose} />
	{/snippet}
	<Segmented options={modeOptions} value={mode} label={t('engine.mode')} onselect={onmode} />
	{#if effects}
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
		<Stack>
			<Slider label={t('engine.settings.haze')} value={effects.haze} min={0} max={6} step={0.05} oninput={(value) => set('haze', value)} />
			<Slider label={t('engine.settings.waterClarity')} value={effects.water_clarity} min={0.25} max={12} step={0.25} oninput={(value) => set('water_clarity', value)} />
			<Slider label={t('engine.settings.bloom')} value={effects.bloom} min={0} max={3} oninput={(value) => set('bloom', value)} />
			<Slider
				label={t('engine.settings.bloomThreshold')}
				value={effects.bloom_threshold}
				min={0.2}
				max={8}
				step={0.05}
				disabled={effects.bloom <= 0}
				oninput={(value) => set('bloom_threshold', value)}
			/>
			<Slider label={t('engine.settings.exposure')} value={effects.exposure} min={0.1} max={4} oninput={(value) => set('exposure', value)} />
			<Select label={t('engine.settings.toneMap')} options={toneMapOptions} value={effects.tone_map} onselect={(value) => set('tone_map', value)} />
		</Stack>
		{#if defaults}
			<Button variant="ghost" type="button" onclick={() => defaults && onchange(defaults)}>{t('engine.settings.defaults')}</Button>
		{/if}
	{/if}
	{#if fps !== undefined}
		<dl>
			<Stat label={t('engine.stats.fps')} value={fps.toFixed(0)} />
		</dl>
	{/if}
</Panel>
