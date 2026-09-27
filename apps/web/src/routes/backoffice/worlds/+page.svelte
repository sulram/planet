<script lang="ts">
	import { Alert, Badge, Button, Dialog, Field, Input, Page, PageHeader, Pager, Stack, Table } from '$lib/ds';
	import { d, t } from '$lib/i18n';
	import type { WorldRow } from '$lib/server/worlds';
	import { WORLD_NAME_MAX } from '$lib/world';
	import type { PageProps } from './$types';

	let { data, form }: PageProps = $props();

	let target = $state<WorldRow>();
	let renaming = $state(false);
	let deleting = $state(false);
	let choosing = $state(false);
	let name = $state('');

	function askRename(world: WorldRow) {
		target = world;
		name = world.name;
		renaming = true;
	}

	function askDelete(world: WorldRow) {
		target = world;
		deleting = true;
	}

	function askMain(world: WorldRow) {
		target = world;
		choosing = true;
	}
</script>

<svelte:head>
	<title>{t('bo.worlds.title')} · {t('common.appName')}</title>
</svelte:head>

<Page>
	<PageHeader
		title={t('bo.worlds.title')}
		lede={t('bo.worlds.lede')}
		trail={[{ label: t('nav.backoffice'), href: '/backoffice' }]}
	>
		{#snippet actions()}
			<Button href="/backoffice/explore">{t('bo.worlds.create')}</Button>
		{/snippet}
	</PageHeader>

	<Stack>
		{#if form && 'error' in form}<Alert variant="danger">{form.error}</Alert>{/if}
		{#if form && 'done' in form}<Alert>{form.done}</Alert>{/if}

		{#if !data.worlds}
			<Alert variant="danger" title={t('error.unavailable.title')}>{t('error.unreachable')}</Alert>
		{:else if data.worlds.items.length === 0}
			<Alert>{t('bo.worlds.empty')}</Alert>
		{:else}
			<Table>
				<table>
					<thead>
						<tr>
							<th>{t('world.name')}</th>
							<th>{t('world.seed')}</th>
							<th>{t('world.generatorVersion')}</th>
							<th>{t('world.owner')}</th>
							<th>{t('world.created')}</th>
							<th></th>
						</tr>
					</thead>
					<tbody>
						{#each data.worlds.items as world (world.id)}
							<tr>
								<td>
									{world.name}
									{#if world.id === data.mainWorld}<Badge variant="solid">{t('bo.worlds.main.badge')}</Badge>{/if}
								</td>
								<td class="muted">{world.recipe.seed}</td>
								<td class="muted">{world.recipe.generator_version}</td>
								<td class="muted">{world.ownerEmail}</td>
								<td class="muted">{d(world.created)}</td>
								<td class="actions">
									<Button variant="ghost" href="/w/{world.id}">{t('world.enter')}</Button>
									{#if world.id !== data.mainWorld}
										<Button variant="ghost" onclick={() => askMain(world)}>{t('bo.worlds.main')}</Button>
									{/if}
									<Button variant="ghost" onclick={() => askRename(world)}>{t('bo.worlds.rename')}</Button>
									<Button variant="danger" onclick={() => askDelete(world)}>{t('bo.worlds.delete')}</Button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</Table>
			<Pager page={data.worlds.page} pages={data.worlds.pages} />
		{/if}
	</Stack>
</Page>

<Dialog bind:open={renaming} title={t('bo.worlds.rename.title')} confirmLabel={t('bo.worlds.rename')} action="?/rename">
	<input type="hidden" name="id" value={target?.id ?? ''} />
	<Field label={t('world.name')} for="rename" hint={t('bo.worlds.rename.hint', { max: WORLD_NAME_MAX })}>
		<Input id="rename" name="name" bind:value={name} maxlength={WORLD_NAME_MAX} autocomplete="off" required />
	</Field>
</Dialog>

<Dialog bind:open={choosing} title={t('bo.worlds.main.title')} confirmLabel={t('bo.worlds.main')} action="?/main">
	<input type="hidden" name="id" value={target?.id ?? ''} />
	<input type="hidden" name="name" value={target?.name ?? ''} />
	<p>{t('bo.worlds.main.body', { name: target?.name ?? '' })}</p>
</Dialog>

<Dialog bind:open={deleting} title={t('bo.worlds.delete.title')} confirmLabel={t('bo.worlds.delete')} danger action="?/delete">
	<input type="hidden" name="id" value={target?.id ?? ''} />
	<p>{t('bo.worlds.delete.body', { name: target?.name ?? '' })}</p>
</Dialog>
