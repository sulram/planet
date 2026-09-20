import { fail, redirect } from '@sveltejs/kit';
import { translate } from '$lib/i18n/config';
import { loginPath } from '$lib/server/auth';
import { pbStatus } from '$lib/server/pb';
import { createWorld } from '$lib/server/worlds';
import { isGeneratorVersion, normalizeSeed, randomSeed, WORLD_NAME_MAX } from '$lib/world';
import type { Actions, PageServerLoad } from './$types';

// The offline preview. The seed lives in the URL so a planet can be shared;
// arriving without a valid one lands on a fresh random planet.
export const load: PageServerLoad = ({ url }) => {
	const seed = normalizeSeed(url.searchParams.get('seed'));
	if (!seed || seed !== url.searchParams.get('seed')) redirect(303, `/play?seed=${seed ?? randomSeed()}`);
	return { seed };
};

export const actions: Actions = {
	// "Create world" writes one row: the previewed recipe, a name and the owner.
	// It runs with the person's own token; PocketBase rules do the rest.
	create: async ({ request, locals }) => {
		const form = await request.formData();
		const name = String(form.get('name') ?? '').trim();
		const seed = normalizeSeed(String(form.get('seed') ?? ''));
		if (!locals.user) redirect(303, loginPath(seed ? `/play?seed=${seed}` : '/play'));

		const generatorVersion = Number(form.get('generator_version'));

		if (!name) return fail(400, { name, error: translate(locals.locale, 'play.error.nameRequired') });
		if (name.length > WORLD_NAME_MAX) {
			return fail(400, { name, error: translate(locals.locale, 'play.error.nameTooLong', { max: WORLD_NAME_MAX }) });
		}
		if (!seed || !isGeneratorVersion(generatorVersion)) {
			return fail(400, { name, error: translate(locals.locale, 'play.error.recipeInvalid') });
		}

		let id: string;
		try {
			({ id } = await createWorld(locals.pb, locals.user.id, name, {
				seed,
				generator_version: generatorVersion,
				params: {}
			}));
		} catch (err) {
			const key = pbStatus(err) === 0 ? 'error.unreachable' : 'play.error.createFailed';
			return fail(502, { name, error: translate(locals.locale, key) });
		}
		redirect(303, `/w/${id}`);
	}
};
