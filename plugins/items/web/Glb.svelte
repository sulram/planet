<script lang="ts">
	import type { KindProps } from './kinds';
	import Model from './Model.svelte';

	// The 3D model kind's own controls: the model kind's, for a `.glb`.
	let props: KindProps = $props();
</script>

<Model {...props} kind="glb" />
