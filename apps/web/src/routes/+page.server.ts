import { error } from '@sveltejs/kit';
import { translate } from '$lib/i18n/config';
import { enterWorld } from '$lib/server/enter';
import { getInstance } from '$lib/server/instance';
import type { PageServerLoad } from './$types';

// The front door: the instance's main world, full screen. Until an operator
// has chosen one the door is closed, and the page says so.
export const load: PageServerLoad = async (event) => {
	let mainWorld: string | null;
	try {
		({ mainWorld } = await getInstance(event.locals.pb));
	} catch {
		error(503, translate(event.locals.locale, 'error.unreachable'));
	}
	if (!mainWorld) return { entry: null };
	return { entry: await enterWorld(event, mainWorld) };
};
