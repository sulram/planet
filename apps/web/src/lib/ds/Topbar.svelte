<script lang="ts">
	import LangSwitch from './LangSwitch.svelte';
	import ThemeToggle from './ThemeToggle.svelte';
	import { t } from '$lib/i18n';

	// The bar of what is off the planet: an error, a world not made yet. The
	// world itself is the whole page and carries no bar. Who a person is and
	// where they sign in belong to mundos, so nothing of an account is here.
</script>

<header class="topbar">
	<a class="brand" href="/">{t('common.appName')}</a>
	<div class="choices">
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
	.choices {
		display: flex;
		align-items: center;
		gap: var(--sp-3);
		margin-left: auto;
	}
</style>
