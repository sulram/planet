<script lang="ts">
	import { Alert, Button, Page, PageHeader, Pager, Table, Topbar } from '$lib/ds';
	import { d, t } from '$lib/i18n';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<svelte:head>
	<title>{t('common.appName')}</title>
</svelte:head>

<Topbar />

<Page>
	<PageHeader title={t('home.title')} lede={t('home.lede')}>
		{#snippet actions()}
			<Button href="/play">{t('home.explore')}</Button>
		{/snippet}
	</PageHeader>

	{#if !data.worlds}
		<Alert variant="danger" title={t('home.unreachable.title')}>{t('home.unreachable.body')}</Alert>
	{:else if data.worlds.items.length === 0}
		<Alert>{t('home.empty')}</Alert>
	{:else}
		<Table>
			<table>
				<thead>
					<tr>
						<th>{t('world.name')}</th>
						<th>{t('world.seed')}</th>
						<th>{t('world.created')}</th>
						<th></th>
					</tr>
				</thead>
				<tbody>
					{#each data.worlds.items as world (world.id)}
						<tr>
							<td><a href="/w/{world.id}">{world.name}</a></td>
							<td class="muted">{world.recipe.seed}</td>
							<td class="muted">{d(world.created)}</td>
							<td class="actions"><Button variant="ghost" href="/w/{world.id}">{t('world.enter')}</Button></td>
						</tr>
					{/each}
				</tbody>
			</table>
		</Table>
		<Pager page={data.worlds.page} pages={data.worlds.pages} />
	{/if}
</Page>
