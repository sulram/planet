<script lang="ts">
	import { Alert, Button, Page, PageHeader, Stack, Topbar } from '$lib/ds';
	import { t } from '$lib/i18n';
</script>

<svelte:head>
	<title>{t('auth.verify.title')} · {t('common.appName')}</title>
</svelte:head>

<Topbar />

<Page size="narrow">
	<PageHeader title={t('auth.verify.title')} />
	<Stack>
		<Alert variant="danger">{t('auth.verify.lede')}</Alert>
		<Button href="/login">{t('auth.verify.requestNew')}</Button>
	</Stack>
</Page>
