<script lang="ts">
	import Stage from '$lib/engine/Stage.svelte';
	import { t } from '$lib/i18n';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<svelte:head>
	<title>{data.world.name} · {t('common.appName')}</title>
</svelte:head>

<Stage title={data.world.name} recipe={data.world.recipe} fieldPath={data.fieldPath ?? undefined} avatar={data.avatar} name={data.name} link={data.link} />
