<script lang="ts">
	// One icon, drawn as a stroke in the current colour at the text size.
	// The paths are Lucide's (ISC): adding one is copying its path data here,
	// so the set stays the handful the product uses and no library rides along.
	export type IconName =
		| 'globe'
		| 'users'
		| 'send'
		| 'map-pin'
		| 'pencil'
		| 'log-out'
		| 'layout-dashboard'
		| 'undo-2'
		| 'redo-2'
		| 'x'
		| 'trash-2'
		| 'settings'
		| 'puzzle'
		| 'languages'
		| 'box'
		| 'image'
		| 'move-3d'
		| 'rotate-3d'
		| 'scale-3d'
		| 'rotate-ccw'
		| 'package'
		| 'lock'
		| 'lock-open'
		| 'list'
		| 'star'
		| 'volume-2';

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
		'map-pin': 'M20 10c0 6-8 12-8 12s-8-6-8-12a8 8 0 0 1 16 0ZM12 7a3 3 0 1 0 0 6 3 3 0 0 0 0-6Z',
		pencil:
			'M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497zm-6.174-1.812 4 4',
		'log-out': 'm16 17 5-5-5-5M21 12H9M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4',
		'undo-2': 'M9 14 4 9l5-5M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5 5.5 5.5 0 0 1-5.5 5.5H11',
		'redo-2': 'm15 14 5-5-5-5M20 9H9.5A5.5 5.5 0 0 0 4 14.5 5.5 5.5 0 0 0 9.5 20H13',
		x: 'M18 6 6 18M6 6l12 12',
		'trash-2': 'M10 11v6M14 11v6M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2',
		// Lucide draws a circle in these; spelled as two arcs here.
		settings:
			'M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915M9 12a3 3 0 1 0 6 0a3 3 0 1 0-6 0',
		puzzle:
			'M15.39 4.39a1 1 0 0 0 1.68-.474 2.5 2.5 0 1 1 3.014 3.015 1 1 0 0 0-.474 1.68l1.683 1.682a2.414 2.414 0 0 1 0 3.414L19.61 15.39a1 1 0 0 1-1.68-.474 2.5 2.5 0 1 0-3.014 3.015 1 1 0 0 1 .474 1.68l-1.683 1.682a2.414 2.414 0 0 1-3.414 0L8.61 19.61a1 1 0 0 0-1.68.474 2.5 2.5 0 1 1-3.014-3.015 1 1 0 0 0 .474-1.68l-1.683-1.682a2.414 2.414 0 0 1 0-3.414L4.39 8.61a1 1 0 0 1 1.68.474 2.5 2.5 0 1 0 3.014-3.015 1 1 0 0 1-.474-1.68l1.683-1.682a2.414 2.414 0 0 1 3.414 0z',
		languages: 'm5 8 6 6m-7 0 6-6 2-3M2 5h12M7 2h1m14 20-5-10-5 10m2-4h6',
		box: 'M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Zm-17.7-1 8.7 5 8.7-5M12 22V12',
		// Lucide draws a rect with a radius of 2 and a circle; spelled as paths here.
		image:
			'M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2ZM7 9a2 2 0 1 0 4 0a2 2 0 1 0-4 0m14 6-3.086-3.086a2 2 0 0 0-2.828 0L6 21',
		// Lucide draws the body as a rect with a radius of 2; spelled as a path.
		lock: 'M5 11h14a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2ZM7 11V7a5 5 0 0 1 10 0v4',
		'lock-open': 'M5 11h14a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2ZM7 11V7a5 5 0 0 1 9.9-1',
		list: 'M3 12h.01M3 18h.01M3 6h.01M8 12h13M8 18h13M8 6h13',
		star: 'M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z',
		'volume-2':
			'M11 4.702a.705.705 0 0 0-1.203-.498L6.413 7.587A1.4 1.4 0 0 1 5.416 8H3a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h2.416a1.4 1.4 0 0 1 .997.413l3.383 3.384A.705.705 0 0 0 11 19.298zM16 9a5 5 0 0 1 0 6M19.364 18.364a9 9 0 0 0 0-12.728',
		'rotate-ccw': 'M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8M3 3v5h5',
		'move-3d': 'M5 3v16h16M5 19l6-6M2 6l3-3 3 3m10 10 3 3-3 3',
		// Lucide's two ellipse arcs, as it draws them.
		'rotate-3d': 'm15.194 13.707 3.814 1.86-1.86 3.814M16.47214 7.52786A5 10 0 1 0 13 21.79796M21.79796 11A10 5 0 1 0 19 15.57071',
		'scale-3d': 'M5 7v11a1 1 0 0 0 1 1h11M5.293 18.707 11 13m6 6a2 2 0 1 0 4 0a2 2 0 1 0-4 0M3 5a2 2 0 1 0 4 0a2 2 0 1 0-4 0',
		// Lucide draws the middle line as a polyline; spelled as a path here.
		package:
			'M11 21.73a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73zM12 22V12M3.29 7 12 12l8.71-5M7.5 4.27l9 5.15',
		// Lucide draws these four as rects with a radius of 1; spelled as paths here.
		'layout-dashboard':
			'M4 3h5a1 1 0 0 1 1 1v7a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1ZM15 3h5a1 1 0 0 1 1 1v3a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1ZM15 12h5a1 1 0 0 1 1 1v7a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1v-7a1 1 0 0 1 1-1ZM4 16h5a1 1 0 0 1 1 1v3a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1v-3a1 1 0 0 1 1-1Z'
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
