import { error } from '@sveltejs/kit';
import { translate } from '$lib/i18n/config';
import { pbStatus } from '$lib/server/pb';
import { getWorld } from '$lib/server/worlds';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, locals }) => {
	try {
		return { world: await getWorld(locals.pb, params.id) };
	} catch (err) {
		if (pbStatus(err) === 404) error(404, translate(locals.locale, 'error.notFound.title'));
		error(503, translate(locals.locale, 'error.unreachable'));
	}
};
