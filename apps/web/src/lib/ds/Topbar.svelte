<script lang="ts">
	import { page } from '$app/state';
	import Button from './Button.svelte';
	import LangSwitch from './LangSwitch.svelte';
	import ThemeToggle from './ThemeToggle.svelte';
	import { t } from '$lib/i18n';

	// Two areas, one component: the site (default) and the backoffice, each
	// with its own navigation. Operators switch through the last nav link.
	interface Props {
		area?: 'site' | 'backoffice';
	}

	let { area = 'site' }: Props = $props();

	const user = $derived(page.data.user);
	const path = $derived(page.url.pathname);
	const links = $derived(
		area === 'backoffice'
			? [
					{ href: '/backoffice/worlds', label: t('nav.worlds') },
					{ href: '/backoffice/users', label: t('nav.users') },
					{ href: '/', label: t('nav.site') }
				]
			: [
					{ href: '/', label: t('nav.worlds') },
					{ href: '/play', label: t('nav.explore') },
					...(user?.operator ? [{ href: '/backoffice', label: t('nav.backoffice') }] : [])
				]
	);
	const loginHref = $derived(`/login?redirect=${encodeURIComponent(path + page.url.search)}`);
</script>

<header class="topbar">
	<a class="brand" href="/">{t('common.appName')}</a>
	{#if area === 'backoffice'}<span class="area">{t('nav.backoffice')}</span>{/if}
	<nav aria-label={t('nav.main')}>
		{#each links as link (link.href)}
			<a href={link.href} aria-current={link.href === path ? 'page' : undefined}>{link.label}</a>
		{/each}
	</nav>
	<div class="account">
		{#if user}
			<span class="who">{user.email}</span>
			<form method="POST" action="/logout">
				<Button type="submit" variant="ghost">{t('auth.logout')}</Button>
			</form>
		{:else}
			<Button variant="ghost" href={loginHref}>{t('auth.login.title')}</Button>
		{/if}
		<LangSwitch />
		<ThemeToggle />
	</div>
</header>

<style>
	.topbar {
		position: sticky;
		top: 0;
		z-index: var(--z-topbar);
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: var(--sp-3) var(--sp-5);
		padding: var(--sp-3) var(--sp-5);
		border-bottom: var(--bw) solid var(--border-strong);
		background: var(--bg);
	}
	.brand {
		font-weight: var(--fw-bold);
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
		text-decoration: none;
	}
	.area {
		padding: 0 var(--sp-2);
		background: var(--action);
		color: var(--action-text);
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
	}
	nav {
		display: flex;
		gap: var(--sp-5);
	}
	nav a {
		color: var(--text-muted);
		text-decoration: none;
	}
	nav a:hover {
		color: var(--text);
	}
	nav a[aria-current='page'] {
		color: var(--text);
		text-decoration: underline;
	}
	.account {
		display: flex;
		align-items: center;
		gap: var(--sp-3);
		margin-left: auto;
	}
	.who {
		color: var(--text-muted);
	}
</style>
