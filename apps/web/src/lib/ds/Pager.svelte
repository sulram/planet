<script lang="ts">
	import { page as current } from '$app/state';
	import Button from './Button.svelte';
	import { t } from '$lib/i18n';

	// Previous and next over the `page` query parameter. Renders nothing when
	// everything fits on one page.
	interface Props {
		page: number;
		pages: number;
	}

	let { page, pages }: Props = $props();

	function href(n: number): string {
		const params = new URLSearchParams(current.url.searchParams);
		if (n <= 1) params.delete('page');
		else params.set('page', String(n));
		const query = params.toString();
		return current.url.pathname + (query ? `?${query}` : '');
	}
</script>

{#if pages > 1}
	<nav class="pager" aria-label={t('common.pagination')}>
		{#if page > 1}<Button variant="ghost" href={href(page - 1)}>{t('common.previous')}</Button>{/if}
		<span>{t('common.pageOf', { page, pages })}</span>
		{#if page < pages}<Button variant="ghost" href={href(page + 1)}>{t('common.next')}</Button>{/if}
	</nav>
{/if}

<style>
	.pager {
		display: flex;
		align-items: center;
		gap: var(--sp-4);
		margin-top: var(--sp-5);
		color: var(--text-muted);
	}
</style>
