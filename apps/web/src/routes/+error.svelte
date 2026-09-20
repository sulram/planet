<script lang="ts">
	import { page } from '$app/state';
	import { Button, Page, PageHeader, Topbar } from '$lib/ds';
	import { t } from '$lib/i18n';

	const kind = $derived(page.status === 404 ? 'notFound' : page.status === 503 ? 'unavailable' : 'generic');
</script>

<svelte:head>
	<title>{page.status} · {t('common.appName')}</title>
</svelte:head>

<Topbar />

<Page size="narrow">
	<PageHeader
		title={t(`error.${kind}.title`)}
		lede={t(`error.${kind}.lede`)}
	/>
	<Button href="/">{t('error.home')}</Button>
</Page>
