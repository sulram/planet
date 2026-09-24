import { fail, redirect } from '@sveltejs/kit';
import { translate } from '$lib/i18n/config';
import { loginPath } from '$lib/server/auth';
import { visitorAvatar } from '$lib/server/avatar';
import { pbStatus } from '$lib/server/pb';
import { createWorld } from '$lib/server/worlds';
import {
	buildParams,
	clampKnob,
	isGeneratorVersion,
	isShape,
	knobsFor,
	normalizeSeed,
	randomSeed,
	WORLD_NAME_MAX,
	type Knobs,
	type Shape
} from '$lib/world';
import { readFieldSidecar } from '$lib/server/fields';
import type { Actions, PageServerLoad } from './$types';

// The offline preview. The seed and the shape live in the URL so a planet can
// be shared; arriving without a valid seed lands on a fresh random planet.
export const load: PageServerLoad = async (event) => {
	const { url, fetch } = event;
	const seed = normalizeSeed(url.searchParams.get('seed'));
	if (!seed || seed !== url.searchParams.get('seed')) redirect(303, `/play?seed=${seed ?? randomSeed()}`);
	const asked = url.searchParams.get('shape');
	const shape: Shape = isShape(asked) ? asked : 'generated';
	// The sidecar names the ground without anyone downloading it: the page can
	// build the whole recipe before the engine fetches a single texel.
	const field = shape === 'generated' ? null : await readFieldSidecar(fetch, shape);
	const settled: Shape = field ? shape : 'generated';
	return {
		seed,
		shape: settled,
		field,
		knobs: readKnobs(settled, (key) => url.searchParams.get(key)),
		avatar: await visitorAvatar(event)
	};
};

/** Every knob of a shape, clamped into its range. Absent is its default. */
function readKnobs(shape: Shape, get: (key: string) => string | null): Knobs {
	const knobs: Knobs = {};
	for (const knob of knobsFor(shape)) {
		const asked = get(knob.key);
		knobs[knob.key] = asked === null ? knob.fallback : clampKnob(knob, asked);
	}
	return knobs;
}

export const actions: Actions = {
	// "Create world" writes one row: the previewed recipe, a name and the owner.
	// It runs with the person's own token; PocketBase rules do the rest.
	create: async ({ request, locals, fetch }) => {
		const form = await request.formData();
		const name = String(form.get('name') ?? '').trim();
		const seed = normalizeSeed(String(form.get('seed') ?? ''));
		if (!locals.user) redirect(303, loginPath(seed ? `/play?seed=${seed}` : '/play'));

		const generatorVersion = Number(form.get('generator_version'));
		const asked = String(form.get('shape') ?? 'generated');
		const shape: Shape = isShape(asked) ? asked : 'generated';
		// The id is read here, from the file this instance serves, and never
		// taken from the form: a recipe may only ever name ground we have.
		const field = shape === 'generated' ? null : await readFieldSidecar(fetch, shape);
		// Clamped here too: the form is a suggestion, the range is the rule.
		const knobs = readKnobs(shape, (key) => form.get(key) as string | null);

		if (!name) return fail(400, { name, error: translate(locals.locale, 'play.error.nameRequired') });
		if (name.length > WORLD_NAME_MAX) {
			return fail(400, { name, error: translate(locals.locale, 'play.error.nameTooLong', { max: WORLD_NAME_MAX }) });
		}
		if (!seed || !isGeneratorVersion(generatorVersion) || (shape !== 'generated' && !field)) {
			return fail(400, { name, error: translate(locals.locale, 'play.error.recipeInvalid') });
		}

		let id: string;
		try {
			({ id } = await createWorld(locals.pb, locals.user.id, name, {
				seed,
				generator_version: generatorVersion,
				params: buildParams(shape, knobs, field)
			}));
		} catch (err) {
			const key = pbStatus(err) === 0 ? 'error.unreachable' : 'play.error.createFailed';
			return fail(502, { name, error: translate(locals.locale, key) });
		}
		redirect(303, `/w/${id}`);
	}
};
