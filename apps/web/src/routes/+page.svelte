<script lang="ts">
	import { Alert, Button, Page, PageHeader, Topbar } from '$lib/ds';
	import Stage from '$lib/engine/Stage.svelte';
	import { t } from '$lib/i18n';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<svelte:head>
	<title>{data.entry ? data.entry.world.name : t('common.appName')}</title>
</svelte:head>

{#if data.entry}
	{@const entry = data.entry}
	<Stage title={entry.world.name} recipe={entry.world.recipe} fieldPath={entry.fieldPath ?? undefined} avatar={entry.avatar} name={entry.name} link={entry.link} />
{:else}
	<Topbar />
	<Page size="narrow">
		<PageHeader title={t('home.closed.title')} lede={t('home.closed.lede')} />
		{#if data.user?.operator}
			<Alert>
				{t('home.closed.choose')}
				<Button href="/backoffice/worlds">{t('nav.worlds')}</Button>
			</Alert>
		{/if}
	</Page>
{/if}
