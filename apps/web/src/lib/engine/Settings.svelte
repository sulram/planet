<script lang="ts">
	import { Checkbox, IconButton, Panel, Segmented, Select, Slider, Stack, Stat } from '$lib/ds';
	import { t } from '$lib/i18n';
	import { toneMaps, type Effects, type Presets } from './index';

	// Settings: how the picture is made, in the panel the gear in the top
	// right corner opens. First the picture whole: light or full, the two
	// presets the engine defines; advanced opens the knobs, in two tabs:
	// how far each kind of thing is drawn (LOD) and the effects (VFX). A
	// front end only: it shows what the engine said is in force and asks for
	// changes; the engine clamps and answers. How the body moves is the
	// keys' alone.
	interface Props {
		/** What the engine last reported. Undefined until it has. */
		effects: Effects | undefined;
		/** The light and the full picture. Undefined until the engine is loaded. */
		presets: Presets | undefined;
		onchange: (effects: Effects) => void;
		/** Frames a second, as the engine last counted them. */
		fps: number | undefined;
		onclose: () => void;
	}

	let { effects, presets, onchange, fps, onclose }: Props = $props();

	type Preset = keyof Presets | 'advanced';
	const pictures: { value: Preset; label: string }[] = [
		{ value: 'light', label: t('engine.settings.light') },
		{ value: 'full', label: t('engine.settings.full') },
		{ value: 'advanced', label: t('engine.settings.advanced') }
	];
	/** Advanced was opened: the knobs show, whatever they stand at. */
	let advanced = $state(false);
	/** The preset the effects in force are, if they are one. */
	const matched = $derived.by((): keyof Presets | null => {
		if (!effects || !presets) return null;
		return same(effects, presets.light) ? 'light' : same(effects, presets.full) ? 'full' : null;
	});
	const picture = $derived<Preset>(advanced ? 'advanced' : (matched ?? 'advanced'));

	function same(a: Effects, b: Effects): boolean {
		return (Object.keys(a) as (keyof Effects)[]).every((key) => a[key] === b[key]);
	}

	function pick(next: Preset) {
		advanced = next === 'advanced';
		if (next !== 'advanced' && presets) onchange(presets[next]);
	}

	type Tab = 'lod' | 'vfx';
	let tab = $state<Tab>('lod');
	const tabs: { value: Tab; label: string }[] = [
		{ value: 'lod', label: 'LOD' },
		{ value: 'vfx', label: 'VFX' }
	];
	const toneMapOptions = $derived(toneMaps.map((value) => ({ value, label: t(`engine.settings.toneMap.${value}`) })));

	function set<K extends keyof Effects>(key: K, value: Effects[K]) {
		if (effects) onchange({ ...effects, [key]: value });
	}
</script>

<Panel title={t('engine.settings')} corner="top-right" under>
	{#snippet aside()}
		<IconButton icon="x" label={t('common.close')} onclick={onclose} />
	{/snippet}
	{#if presets}
		<Segmented options={pictures} value={picture} label={t('engine.settings.picture')} onselect={pick} />
	{/if}
	{#if picture === 'advanced'}
		<Segmented options={tabs} value={tab} label={t('engine.settings.tabs')} onselect={(value) => (tab = value)} />
	{/if}
	{#if effects && picture === 'advanced' && tab === 'vfx'}
		<Stack>
			<Slider
				label={t('engine.settings.resolution')}
				value={effects.resolution}
				min={0.25}
				max={1}
				step={0.25}
				format={(n) => t('engine.settings.percent', { n: (n * 100).toFixed(0) })}
				oninput={(value) => set('resolution', value)}
			/>
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
	{/if}
	{#if effects && picture === 'advanced' && tab === 'lod'}
		<Stack>
			<Slider
				label={t('engine.settings.videoPlay')}
				value={effects.video_play_m}
				min={16}
				max={1024}
				step={16}
				format={(n) => t('engine.stats.metres', { n: n.toFixed(0) })}
				oninput={(value) => set('video_play_m', value)}
			/>
			<Slider
				label={t('engine.settings.pictureReach')}
				value={effects.picture_reach_m}
				min={32}
				max={2048}
				step={32}
				format={(n) => t('engine.stats.metres', { n: n.toFixed(0) })}
				oninput={(value) => set('picture_reach_m', value)}
			/>
			<Slider
				label={t('engine.settings.modelReach')}
				value={effects.model_reach_m}
				min={16}
				max={1024}
				step={16}
				format={(n) => t('engine.settings.metresABlock', { n: n.toFixed(0) })}
				oninput={(value) => set('model_reach_m', value)}
			/>
		</Stack>
	{/if}
	{#if fps !== undefined}
		<dl>
			<Stat label={t('engine.stats.fps')} value={fps.toFixed(0)} />
		</dl>
	{/if}
</Panel>
