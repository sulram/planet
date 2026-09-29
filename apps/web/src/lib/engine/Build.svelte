<script lang="ts">
	import { Alert, Button, Panel, Segmented } from '$lib/ds';
	import { t } from '$lib/i18n';
	import { platforms, tools, type BuildRefusal, type Tool } from './index';

	// Building: a button in the bottom right corner, and the panel it opens
	// while a tool is in hand. A front-end only: it shows the tool, the paint
	// and the platform the engine says are picked and asks for others; where a
	// volume opens, what a platform is and what a stroke does are the engine's.
	interface Props {
		/** The tool in hand, null when not building. */
		tool: Tool | null;
		/** Index into `palette`. */
		paint: number;
		/** The side of the platform laid next, in cells. */
		platform: number;
		/** The paints a cell can take, as `#rrggbb`, in order. */
		palette: string[];
		/** Why the last platform asked for could not be laid, until the next one. */
		refused: BuildRefusal | null;
		/** Whether there is a stroke to take back, and one to put back. */
		history: { undo: boolean; redo: boolean };
		onbuild: (tool: Tool | null) => void;
		onpaint: (paint: number) => void;
		onplatform: (side: number) => void;
		/** Lay a platform where the body stands. */
		onlay: () => void;
		onundo: () => void;
		onredo: () => void;
	}

	let { tool, paint, platform, palette, refused, history, onbuild, onpaint, onplatform, onlay, onundo, onredo }: Props = $props();

	const toolOptions = $derived(tools.map((value) => ({ value, label: t(`engine.build.tool.${value}`) })));
	const platformOptions = platforms.map((side) => ({ value: String(side), label: String(side) }));

	// Building again takes the tool put down last, as B does in the engine.
	let last = $state<Tool>('create');
	$effect(() => {
		if (tool) last = tool;
	});
</script>

{#if tool}
	<Panel title={t('engine.build')} corner="bottom-right">
		{#snippet aside()}
			<button class="close" type="button" onclick={() => onbuild(null)}>{t('common.close')}</button>
		{/snippet}
		<Segmented options={toolOptions} value={tool} label={t('engine.build.tool')} onselect={(value) => onbuild(value)} />
		<div class="platform">
			<span>{t('engine.build.platform')}</span>
			<Segmented
				options={platformOptions}
				value={String(platform)}
				label={t('engine.build.platform')}
				onselect={(value) => onplatform(Number(value))}
			/>
		</div>
		<Button variant="ghost" type="button" onclick={onlay}>{t('engine.build.platform.lay')}</Button>
		{#if refused}
			<Alert variant="info">{t(`engine.build.refused.${refused}`)}</Alert>
		{/if}
		<div class="palette" role="group" aria-label={t('engine.build.paint')}>
			{#each palette as color, index (index)}
				<button
					type="button"
					class="swatch"
					style:background={color}
					aria-pressed={index === paint}
					aria-label={t('engine.build.paint.pick', { n: String(index + 1) })}
					onclick={() => onpaint(index)}
				></button>
			{/each}
		</div>
		<div class="history">
			<Button variant="ghost" type="button" disabled={!history.undo} onclick={onundo}>{t('engine.build.undo')}</Button>
			<Button variant="ghost" type="button" disabled={!history.redo} onclick={onredo}>{t('engine.build.redo')}</Button>
		</div>
		<p class="hint">{t('engine.build.hint')}</p>
	</Panel>
{:else}
	<div class="toggle">
		<Button variant="ghost" type="button" onclick={() => onbuild(last)}>{t('engine.build')}</Button>
	</div>
{/if}

<style>
	.toggle {
		position: absolute;
		right: var(--sp-4);
		bottom: var(--sp-5);
		z-index: var(--z-panel);
	}
	.toggle :global(button) {
		background: var(--bg-overlay);
	}
	.platform {
		display: flex;
		align-items: center;
		gap: var(--sp-2);
		color: var(--text-muted);
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
	.palette {
		display: grid;
		grid-template-columns: repeat(8, 1fr);
		gap: var(--sp-1);
	}
	.swatch {
		aspect-ratio: 1;
		border: var(--bw) solid var(--border);
		cursor: pointer;
	}
	.swatch[aria-pressed='true'] {
		outline: var(--bw) solid var(--text);
		outline-offset: var(--bw);
	}
	.history {
		display: flex;
		gap: var(--sp-2);
	}
	.hint {
		color: var(--text-muted);
	}
</style>
