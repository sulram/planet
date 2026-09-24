<script lang="ts">
	import { Stat } from '$lib/ds';
	import Stage from '$lib/engine/Stage.svelte';
	import { t } from '$lib/i18n';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<svelte:head>
	<title>{data.world.name} · {t('common.appName')}</title>
</svelte:head>

<Stage title={data.world.name} recipe={data.world.recipe} fieldPath={data.fieldPath ?? undefined} avatar={data.avatar} link={data.link}>
	<dl>
		<Stat label={t('world.seed')} value={data.world.recipe.seed} />
		<Stat label={t('world.generatorVersion')} value={String(data.world.recipe.generator_version)} />
	</dl>
</Stage>
