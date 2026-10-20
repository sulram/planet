<script lang="ts">
	import type { KindProps } from './kinds';
	import Picture from './Picture.svelte';

	// The video kind's own controls: the picture's, for an MP4. A video is
	// placed as a picture is and heard within its reach.
	let props: KindProps = $props();
</script>

<Picture {...props} item="video" />
