<script lang="ts" generics="T extends string">
	// One of a few named choices, where a Segmented row would not fit. The
	// platform's own select: keyboard, touch and screen readers come with it.
	interface Option {
		value: T;
		label: string;
	}
	interface Props {
		label: string;
		options: readonly Option[];
		value: T;
		disabled?: boolean;
		onselect?: (value: T) => void;
	}

	let { label, options, value, disabled = false, onselect }: Props = $props();
</script>

<label class="select">
	<span>{label}</span>
	<select {value} {disabled} onchange={(e) => onselect?.(e.currentTarget.value as T)}>
		{#each options as opt (opt.value)}
			<option value={opt.value}>{opt.label}</option>
		{/each}
	</select>
</label>

<style>
	.select {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--sp-3);
	}
	select {
		min-height: var(--control-h);
		padding: 0 var(--sp-3);
		border: var(--bw) solid var(--border-strong);
		border-radius: var(--radius);
		background: var(--bg);
		color: var(--text);
		font: inherit;
		cursor: pointer;
	}
	select:disabled {
		color: var(--text-muted);
		cursor: not-allowed;
	}
	select:focus-visible {
		outline: var(--bw) solid var(--focus);
		outline-offset: var(--sp-1);
	}
</style>
