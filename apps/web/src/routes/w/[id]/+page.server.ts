import { error } from '@sveltejs/kit';
import { translate } from '$lib/i18n/config';
import { visitorAvatar } from '$lib/server/avatar';
import { pbStatus } from '$lib/server/pb';
import { getWorld } from '$lib/server/worlds';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async (event) => {
	const { params, locals } = event;
	try {
		return { world: await getWorld(locals.pb, params.id), avatar: await visitorAvatar(event) };
	} catch (err) {
		if (pbStatus(err) === 404) error(404, translate(locals.locale, 'error.notFound.title'));
		error(503, translate(locals.locale, 'error.unreachable'));
	}
};
