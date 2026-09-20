<script lang="ts">
	// One labelled value. A group of them sits inside a <dl>.
	interface Props {
		label: string;
		value: string;
	}

	let { label, value }: Props = $props();
</script>

<div class="stat">
	<dt>{label}</dt>
	<dd>{value}</dd>
</div>

<style>
	.stat {
		display: flex;
		justify-content: space-between;
		gap: var(--sp-3);
	}
	dt {
		color: var(--text-muted);
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
	}
	dd {
		font-variant-numeric: tabular-nums;
		overflow-wrap: anywhere;
		text-align: right;
	}
</style>
