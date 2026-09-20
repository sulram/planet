import { pageParam } from '$lib/server/pb';
import { listWorlds } from '$lib/server/worlds';
import type { PageServerLoad } from './$types';

// The public list of worlds. PocketBase down is a state of the page, not a 500.
export const load: PageServerLoad = async ({ locals, url }) => {
	try {
		return { worlds: await listWorlds(locals.pb, pageParam(url)) };
	} catch {
		return { worlds: null };
	}
};
