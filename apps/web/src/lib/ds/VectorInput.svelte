<script lang="ts">
	// A vector of numbers, one field an axis, each a scrubber too: dragging
	// a field sideways changes its value, clicking it types one. Shift
	// multiplies the step by ten. The way an editor's inspector says a
	// position, a rotation or a size, after tatil-tim-platform. With
	// `onreset`, a small button beside the label puts the vector back. With
	// `lockLabel`, a lock beside it keeps the proportion: one axis changed
	// changes the others by the same factor.
	import Icon from './Icon.svelte';

	interface Props {
		/** Accessible name of the group, shown before the fields. */
		label: string;
		/** The axes, in order: their names and their values. */
		axes: readonly string[];
		values: readonly number[];
		/** How much one pixel of drag, and a typed value's rounding, is worth. */
		step?: number;
		/** What a drag steps by with Shift held: ten steps when unsaid. A coarse step, like a rotation's 15 degrees, makes Shift the fine one. */
		fine?: number;
		/** Digits after the point, as shown. */
		decimals?: number;
		min?: number;
		max?: number;
		/** The vector as it is now, after any one axis changed. */
		onchange: (values: number[]) => void;
		/** Puts the vector back to what it was: shown as a button beside the label. */
		onreset?: () => void;
		/** The reset button's name. Required with `onreset`. */
		resetLabel?: string;
		/** The lock's name: with it, the lock is offered. */
		lockLabel?: string;
		/** Whether the lock starts closed. */
		locked?: boolean;
	}

	let { label, axes, values, step = 0.1, fine, decimals = 2, min = -Infinity, max = Infinity, onchange, onreset, resetLabel, lockLabel, locked: lockedAtFirst = false }: Props =
		$props();

	// svelte-ignore state_referenced_locally
	let locked = $state(lockedAtFirst);

	/** Steps a pixel of drag is worth. */
	const SENSITIVITY = 0.5;
	/** Pixels before a press becomes a drag; under it, a click to type. */
	const THRESHOLD = 3;

	let dragging = $state<number | null>(null);
	let pending: number | null = null;
	let startX = 0;
	let startValue = 0;
	/** The field being typed in, and its text. */
	let typing = $state<number | null>(null);
	let draft = $state('');

	const clamp = (n: number) => Math.min(max, Math.max(min, n));
	const show = (n: number) => n.toFixed(decimals);

	function set(axis: number, value: number) {
		const next = [...values];
		const was = values[axis];
		next[axis] = clamp(Number(value.toFixed(decimals)));
		// Locked, the others follow by the same factor, when there is one.
		if (locked && lockLabel && was !== 0 && Number.isFinite(next[axis] / was)) {
			const factor = next[axis] / was;
			for (let other = 0; other < next.length; other++) {
				if (other !== axis) next[other] = clamp(Number((values[other] * factor).toFixed(decimals)));
			}
		}
		onchange(next);
	}

	function down(axis: number, event: PointerEvent) {
		if (typing !== null) return;
		pending = axis;
		startX = event.clientX;
		startValue = values[axis];
		(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
	}

	function move(event: PointerEvent) {
		if (pending !== null && dragging === null && Math.abs(event.clientX - startX) > THRESHOLD) {
			dragging = pending;
			pending = null;
		}
		if (dragging === null) return;
		const steps = Math.round((event.clientX - startX) * SENSITIVITY);
		set(dragging, startValue + steps * (event.shiftKey ? (fine ?? step * 10) : step));
	}

	function up(axis: number, event: PointerEvent) {
		(event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
		if (dragging !== null) {
			dragging = null;
			return;
		}
		if (pending === axis) {
			pending = null;
			typing = axis;
			draft = show(values[axis]);
			requestAnimationFrame(() => {
				const input = (event.target as HTMLElement).closest('.axis')?.querySelector('input');
				input?.focus();
				input?.select();
			});
		}
	}

	function commit() {
		if (typing === null) return;
		const value = Number(draft.replace(',', '.'));
		if (Number.isFinite(value)) set(typing, value);
		typing = null;
	}

	function key(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault();
			commit();
		} else if (event.key === 'Escape') {
			event.preventDefault();
			typing = null;
		}
	}
</script>

<div class="vector" role="group" aria-label={label}>
	<span class="label">
		{label}
		<span class="tools">
			{#if lockLabel}
				<button type="button" class="tool" aria-label={lockLabel} title={lockLabel} aria-pressed={locked} onclick={() => (locked = !locked)}>
					<Icon name={locked ? 'lock' : 'lock-open'} />
				</button>
			{/if}
			{#if onreset}
				<button type="button" class="tool" aria-label={resetLabel} title={resetLabel} onclick={onreset}>
					<Icon name="rotate-ccw" />
				</button>
			{/if}
		</span>
	</span>
	{#each axes as axis, index (axis)}
		<div class="axis" class:dragging={dragging === index}>
			<span class="name">{axis}</span>
			{#if typing === index}
				<input class="field" type="text" inputmode="decimal" aria-label="{label} {axis}" bind:value={draft} onblur={commit} onkeydown={key} />
			{:else}
				<button
					type="button"
					class="field"
					aria-label="{label} {axis}"
					onpointerdown={(event) => down(index, event)}
					onpointermove={move}
					onpointerup={(event) => up(index, event)}
					onpointercancel={() => ((dragging = null), (pending = null))}
				>
					{show(values[index])}
				</button>
			{/if}
		</div>
	{/each}
</div>

<style>
	/* The label on a line of its own, the fields in a row under it: each
	   field has room for a sector's worth of blocks with its decimals. */
	.vector {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		align-items: center;
		gap: var(--sp-2);
		color: var(--text-muted);
	}
	.label {
		grid-column: 1 / -1;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	.tools {
		display: flex;
	}
	.tool {
		display: inline-grid;
		place-items: center;
		width: var(--control-h);
		height: var(--control-h);
		border: none;
		background: none;
		color: var(--text-muted);
		cursor: pointer;
	}
	.tool:hover,
	.tool[aria-pressed='true'] {
		color: var(--text);
	}
	.axis {
		position: relative;
		display: flex;
		align-items: center;
		min-width: 0;
		border: var(--bw) solid var(--border-strong);
		background: var(--bg);
	}
	.axis.dragging {
		border-color: var(--focus);
	}
	.name {
		padding-inline: var(--sp-2);
		color: var(--text-muted);
	}
	.field {
		flex: 1;
		min-width: 0;
		min-height: calc(var(--control-h) - 2 * var(--bw));
		padding: 0 var(--sp-2) 0 0;
		border: none;
		background: none;
		color: var(--text);
		font: inherit;
		text-align: right;
	}
	button.field {
		cursor: ew-resize;
		touch-action: none;
		user-select: none;
	}
	input.field {
		outline: none;
	}
</style>
