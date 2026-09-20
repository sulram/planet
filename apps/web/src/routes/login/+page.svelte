<script lang="ts">
	import { enhance } from '$app/forms';
	import { Button, Field, Input, Page, PageHeader, Stack, Topbar } from '$lib/ds';
	import { t } from '$lib/i18n';
	import type { PageProps } from './$types';

	let { data, form }: PageProps = $props();
	let sending = $state(false);
</script>

<svelte:head>
	<title>{t('auth.login.title')} · {t('common.appName')}</title>
</svelte:head>

<Topbar />

<Page size="narrow">
	<PageHeader title={t('auth.login.title')} lede={t('auth.login.lede')} />
	<form
		method="POST"
		use:enhance={() => {
			sending = true;
			return async ({ update }) => {
				await update();
				sending = false;
			};
		}}
	>
		<input type="hidden" name="redirect" value={data.redirect} />
		<Stack>
			<Field label={t('auth.email')} for="email" error={form?.error}>
				<Input
					id="email"
					name="email"
					type="email"
					autocomplete="email"
					autocapitalize="off"
					spellcheck="false"
					value={form?.email ?? ''}
					invalid={!!form?.error}
					required
				/>
			</Field>
			<Button type="submit" loading={sending}>{t('auth.login.submit')}</Button>
		</Stack>
	</form>
</Page>
