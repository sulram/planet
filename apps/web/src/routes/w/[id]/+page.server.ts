import { error } from '@sveltejs/kit';
import { translate } from '$lib/i18n/config';
import { visitorAvatar } from '$lib/server/avatar';
import { pbStatus } from '$lib/server/pb';
import { servedField } from '$lib/server/fields';
import { getWorld } from '$lib/server/worlds';
import { fieldId } from '$lib/world';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async (event) => {
	const { params, locals } = event;
	try {
		const world = await getWorld(locals.pb, params.id);
		return {
			world,
			// A world shaped by ground this instance does not have cannot be
			// entered: the engine says so rather than showing another planet.
			fieldPath: await servedField(fieldId(world.recipe)),
			avatar: await visitorAvatar(event)
		};
	} catch (err) {
		if (pbStatus(err) === 404) error(404, translate(locals.locale, 'error.notFound.title'));
		error(503, translate(locals.locale, 'error.unreachable'));
	}
};
