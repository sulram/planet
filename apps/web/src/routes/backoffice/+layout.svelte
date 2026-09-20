<script lang="ts">
	import { Topbar } from '$lib/ds';
	import type { LayoutProps } from './$types';

	// Operators only. The barrier is in hooks.server.ts, where it also covers
	// form actions; nothing here needs to check again.
	let { children }: LayoutProps = $props();
</script>

<Topbar area="backoffice" />

{@render children()}
