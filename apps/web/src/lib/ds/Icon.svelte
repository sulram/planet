<script lang="ts">
	// One icon, drawn as a stroke in the current colour at the text size.
	// The paths are Lucide's (ISC): adding one is copying its path data here,
	// so the set stays the handful the product uses and no library rides along.
	export type IconName = 'globe' | 'users' | 'send' | 'map-pin';

	interface Props {
		name: IconName;
		/** Read by assistive tech. Without it the icon is decoration beside a label. */
		label?: string;
	}

	let { name, label }: Props = $props();

	const paths: Record<IconName, string> = {
		globe: 'M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Zm0 0a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20ZM2 12h20',
		users: 'M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2M9 3a4 4 0 1 0 0 8 4 4 0 0 0 0-8Zm13 18v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75',
		send: 'm22 2-7 20-4-9-9-4ZM22 2 11 13',
		'map-pin': 'M20 10c0 6-8 12-8 12s-8-6-8-12a8 8 0 0 1 16 0ZM12 7a3 3 0 1 0 0 6 3 3 0 0 0 0-6Z'
	};
</script>

<svg
	class="icon"
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width="2"
	stroke-linecap="round"
	stroke-linejoin="round"
	role={label ? 'img' : undefined}
	aria-label={label}
	aria-hidden={label ? undefined : true}
>
	<path d={paths[name]} />
</svg>

<style>
	.icon {
		width: 1.4em;
		height: 1.4em;
		flex: none;
		vertical-align: -0.35em;
	}
</style>
