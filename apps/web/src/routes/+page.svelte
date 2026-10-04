<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { wornAvatar } from '$lib/avatar';
	import { Button, Page, PageHeader, Spinner, Topbar } from '$lib/ds';
	import { arrival, pass } from '$lib/door';
	import Found from '$lib/engine/Found.svelte';
	import Stage from '$lib/engine/Stage.svelte';
	import { servedField } from '$lib/fields';
	import { t } from '$lib/i18n';
	import { about, enter, found, Refused, socket, stands, type About, type Entered, type Link } from '$lib/instance';
	import { kept } from '$lib/kept';
	import { fieldId, type Recipe } from '$lib/world';

	// The one page of a world. It asks the world what it is, passes through
	// mundos's door when there is one, and then shows one of three faces: the
	// world, the founding of a world that awaits its planet, to its admin, or
	// word that it is on its way, to everyone else (DECISIONS 87).
	type Face = 'entering' | 'unreachable' | 'unfounded' | 'founding' | 'world';

	let face = $state<Face>('entering');
	let world = $state.raw<About>();
	let me = $state.raw<Entered>();
	let avatar = $state<string | null>(null);
	/** Where the field the recipe names is fetched from; null when this version lacks that ground. */
	let fieldPath = $state<string | null>(null);
	/** Why the last founding failed, as the server codes it. */
	let refused = $state('');
	let founding = $state(false);

	const link = $derived.by<Link | undefined>(() => {
		const key = me?.key ?? '';
		const door = world?.door ?? '';
		return {
			url: socket(key),
			// The socket opens for a key the server holds. Past its life the
			// page goes through the door for a new one.
			again: async () => {
				if (key && door && !(await stands(key))) pass(door);
			}
		};
	});
	const signIn = $derived(world?.door && me?.level === 'anonymous' ? () => pass(world!.door) : undefined);

	onMount(() => {
		arrive();
	});

	async function arrive() {
		try {
			const here = await about();
			let identity = '';
			if (here.door) {
				const came = arrival();
				if (!came) return pass(here.door);
				identity = came.identity;
				// The token leaves the address bar, and the address the page
				// left with comes back: the place, or a planet being chosen.
				await goto(came.address, { replaceState: true, keepFocus: true, noScroll: true });
			}
			[me, avatar] = await Promise.all([enter(identity), wornAvatar()]);
			await show(here);
		} catch {
			face = 'unreachable';
		}
	}

	async function show(here: About) {
		world = here;
		if (!here.recipe) {
			face = me?.level === 'admin' ? 'founding' : 'unfounded';
			return;
		}
		fieldPath = await servedField(fieldId(here.recipe));
		face = 'world';
	}

	async function foundWith(recipe: Recipe) {
		refused = '';
		founding = true;
		try {
			const made = await found(recipe, me?.key ?? '');
			// The candidate's address was the founding's: the world's own is bare.
			await goto('/', { replaceState: true });
			await show(made);
		} catch (err) {
			const code = err instanceof Refused ? err.code : 'unreachable';
			// Someone founded it first: this is that world now.
			if (code === 'founded') await arrive();
			else if (code === 'key' && world?.door) pass(world.door);
			else refused = code;
		} finally {
			founding = false;
		}
	}
</script>

<svelte:head>
	<title>{world?.name || t('common.appName')}</title>
</svelte:head>

{#if face === 'world' && world?.recipe && me}
	<Stage
		title={world.name || t('common.appName')}
		recipe={world.recipe}
		fieldPath={fieldPath ?? undefined}
		{avatar}
		name={me.name || (kept('name') ?? '')}
		{link}
		level={me.level}
		account={me.key !== ''}
		onsignin={signIn}
	/>
{:else if face === 'founding'}
	<Found {avatar} error={refused} {founding} onfound={foundWith} />
{:else}
	<Topbar />
	<Page size="narrow">
		{#if face === 'entering'}
			<Spinner label={t('common.loading')} />
		{:else if face === 'unfounded'}
			<PageHeader title={t('world.unfounded.title')} lede={t('world.unfounded.lede')} />
			{#if signIn}<Button onclick={signIn}>{t('door.signIn')}</Button>{/if}
		{:else}
			<PageHeader title={t('error.unavailable.title')} lede={t('error.unreachable')} />
			<Button onclick={() => location.reload()}>{t('error.retry')}</Button>
		{/if}
	</Page>
{/if}
