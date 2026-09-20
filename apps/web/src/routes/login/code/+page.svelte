<script lang="ts">
	import { enhance } from '$app/forms';
	import { Alert, Button, Field, Input, Page, PageHeader, Stack, Topbar } from '$lib/ds';
	import { t } from '$lib/i18n';
	import type { PageProps } from './$types';

	let { data, form }: PageProps = $props();
	let busy = $state<'verify' | 'resend' | null>(null);
	const error = $derived(form && 'error' in form ? form.error : undefined);
	const resent = $derived(form && 'resent' in form);
</script>

<svelte:head>
	<title>{t('auth.code.title')} · {t('common.appName')}</title>
</svelte:head>

<Topbar />

<Page size="narrow">
	<PageHeader title={t('auth.code.title')} lede={t('auth.code.lede', { email: data.email })} />
	<Stack>
		{#if resent}<Alert>{t('auth.code.resent')}</Alert>{/if}
		<form
			method="POST"
			action="?/verify"
			use:enhance={() => {
				busy = 'verify';
				return async ({ update }) => {
					await update();
					busy = null;
				};
			}}
		>
			<Stack>
				<Field label={t('auth.code.label')} for="code" hint={t('auth.code.hint')} {error}>
					<!-- svelte-ignore a11y_autofocus -->
					<Input
						id="code"
						name="code"
						inputmode="numeric"
						autocomplete="one-time-code"
						autocapitalize="off"
						spellcheck="false"
						invalid={!!error}
						required
						autofocus
					/>
				</Field>
				<Button type="submit" loading={busy === 'verify'}>{t('auth.code.submit')}</Button>
			</Stack>
		</form>
		<form
			method="POST"
			action="?/resend"
			use:enhance={() => {
				busy = 'resend';
				return async ({ update }) => {
					await update();
					busy = null;
				};
			}}
		>
			<Stack>
				<Button type="submit" variant="ghost" loading={busy === 'resend'}>{t('auth.code.resend')}</Button>
				<a href="/login">{t('auth.code.otherEmail')}</a>
			</Stack>
		</form>
	</Stack>
</Page>
