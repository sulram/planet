<script lang="ts">
	import type { ActionResult } from '@sveltejs/kit';
	import { Alert, CodeInput, Dialog, Field, Input, Stack } from '$lib/ds';
	import { t } from '$lib/i18n';

	// Signing in without leaving the world: the same two actions the login
	// pages post to, read here step by step (DECISIONS 70). The email step
	// asks /login for a code; the code step hands it to /login/code. Both keep
	// the pending request in the httpOnly cookie those pages use, so nothing
	// about the flow is new, only where the person stands.
	interface Props {
		open?: boolean;
		/** Called when the person is signed in: the page then reloads itself. */
		ondone?: () => void;
	}

	let { open = $bindable(false), ondone }: Props = $props();

	let step = $state<'email' | 'code'>('email');
	let email = $state('');
	let code = $state('');
	let error = $state('');
	let resent = $state(false);

	const failure = (result: ActionResult): string =>
		result.type === 'failure' && typeof result.data?.error === 'string' ? result.data.error : t('auth.error.unreachable');

	// The email step is done when /login sends the person to the code page.
	async function sent(result: ActionResult): Promise<boolean> {
		if (result.type === 'redirect') {
			step = 'code';
			error = '';
			return false;
		}
		error = failure(result);
		return false;
	}

	// The code step is done when /login/code sends the person home: the
	// session cookie is on that answer.
	async function verified(result: ActionResult): Promise<boolean> {
		if (result.type === 'redirect') {
			ondone?.();
			return true;
		}
		error = failure(result);
		return false;
	}

	async function resend() {
		error = '';
		resent = false;
		const answer = await fetch('/login/code?/resend', {
			method: 'POST',
			headers: { 'x-sveltekit-action': 'true' },
			body: new FormData()
		})
			.then((r) => r.json() as Promise<ActionResult>)
			.catch(() => null);
		if (answer?.type === 'success') resent = true;
		else error = answer ? failure(answer) : t('auth.error.unreachable');
	}

	function otherEmail() {
		step = 'email';
		code = '';
		error = '';
		resent = false;
	}

	$effect(() => {
		if (!open) otherEmail();
	});
</script>

{#if step === 'email'}
	<Dialog bind:open title={t('auth.login.title')} confirmLabel={t('auth.login.submit')} action="/login" onresult={sent}>
		<input type="hidden" name="redirect" value="/" />
		<Stack>
			<p>{t('auth.login.lede')}</p>
			<Field label={t('auth.email')} for="signin-email" {error}>
				<Input id="signin-email" name="email" type="email" bind:value={email} autocomplete="email" autocapitalize="off" spellcheck="false" invalid={!!error} required />
			</Field>
		</Stack>
	</Dialog>
{:else}
	<Dialog bind:open title={t('auth.code.title')} confirmLabel={t('auth.code.submit')} action="/login/code?/verify" onresult={verified}>
		<Stack>
			<p>{t('auth.code.lede', { email })}</p>
			{#if resent}<Alert>{t('auth.code.resent')}</Alert>{/if}
			<Field label={t('auth.code.label')} for="signin-code" hint={t('auth.code.hint')} {error}>
				<CodeInput id="signin-code" name="code" bind:value={code} invalid={!!error} required />
			</Field>
			<p class="links">
				<button type="button" class="link" onclick={resend}>{t('auth.code.resend')}</button>
				<button type="button" class="link" onclick={otherEmail}>{t('auth.code.otherEmail')}</button>
			</p>
		</Stack>
	</Dialog>
{/if}

<style>
	.links {
		display: flex;
		gap: var(--sp-4);
	}
	.link {
		border: none;
		background: none;
		padding: 0;
		color: var(--text);
		text-decoration: underline;
		cursor: pointer;
	}
</style>
