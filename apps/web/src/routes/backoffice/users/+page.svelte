<script lang="ts">
	import { Alert, Badge, Button, Dialog, Page, PageHeader, Pager, Stack, Table } from '$lib/ds';
	import { d, t } from '$lib/i18n';
	import type { UserRow } from '$lib/server/users';
	import type { PageProps } from './$types';

	let { data, form }: PageProps = $props();

	let target = $state<UserRow>();
	let asking = $state(false);

	function ask(user: UserRow) {
		target = user;
		asking = true;
	}
</script>

<svelte:head>
	<title>{t('bo.users.title')} · {t('common.appName')}</title>
</svelte:head>

<Page>
	<PageHeader
		title={t('bo.users.title')}
		lede={t('bo.users.lede')}
		trail={[{ label: t('nav.backoffice'), href: '/backoffice' }]}
	/>

	<Stack>
		{#if form && 'error' in form}<Alert variant="danger">{form.error}</Alert>{/if}
		{#if form && 'done' in form}<Alert>{form.done}</Alert>{/if}

		{#if !data.users}
			<Alert variant="danger" title={t('error.unavailable.title')}>{t('error.unreachable')}</Alert>
		{:else}
			<Table>
				<table>
					<thead>
						<tr>
							<th>{t('auth.email')}</th>
							<th>{t('bo.users.name')}</th>
							<th>{t('bo.users.verified')}</th>
							<th>{t('bo.users.operator')}</th>
							<th>{t('bo.users.created')}</th>
							<th></th>
						</tr>
					</thead>
					<tbody>
						{#each data.users.items as user (user.id)}
							<tr>
								<td>{user.email}</td>
								<td class="muted">{user.name}</td>
								<td>
									<Badge variant={user.verified ? 'solid' : 'outline'}>{t(user.verified ? 'common.yes' : 'common.no')}</Badge>
								</td>
								<td>
									<Badge variant={user.operator ? 'solid' : 'outline'}>{t(user.operator ? 'common.yes' : 'common.no')}</Badge>
								</td>
								<td class="muted">{d(user.created)}</td>
								<td class="actions">
									{#if user.id === data.user?.id}
										<span class="you">{t('bo.users.you')}</span>
									{:else if user.operator}
										<Button variant="danger" onclick={() => ask(user)}>{t('bo.users.remove')}</Button>
									{:else}
										<Button variant="ghost" onclick={() => ask(user)}>{t('bo.users.grant')}</Button>
									{/if}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</Table>
			<Pager page={data.users.page} pages={data.users.pages} />
		{/if}
	</Stack>
</Page>

<Dialog
	bind:open={asking}
	title={t(target?.operator ? 'bo.users.remove.title' : 'bo.users.grant.title')}
	confirmLabel={t(target?.operator ? 'bo.users.remove' : 'bo.users.grant')}
	danger={target?.operator}
	action="?/operator"
>
	<input type="hidden" name="id" value={target?.id ?? ''} />
	<input type="hidden" name="operator" value={String(!target?.operator)} />
	<p>{t(target?.operator ? 'bo.users.remove.body' : 'bo.users.grant.body', { email: target?.email ?? '' })}</p>
</Dialog>

<style>
	.you {
		color: var(--text-muted);
	}
</style>
