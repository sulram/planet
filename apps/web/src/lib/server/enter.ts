import { error, type RequestEvent } from '@sveltejs/kit';
import { translate } from '$lib/i18n/config';
import { visitorAvatar } from './avatar';
import { servedField } from './fields';
import { visitorName } from './name';
import { pbStatus } from './pb';
import { worldLink } from './session';
import { getWorld } from './worlds';
import { fieldId, type World } from '$lib/world';

/** What a page needs to stand someone in a world: `/` and `/w/[id]` share it. */
export interface Entry {
	world: World;
	/** Where the field the recipe names is served from; null when this instance lacks that ground. */
	fieldPath: string | null;
	avatar: string | null;
	name: string;
	link: ReturnType<typeof worldLink>;
}

/** The entry to a world by id. 404 when there is no such world, 503 when the server is away. */
export async function enterWorld(event: RequestEvent, id: string): Promise<Entry> {
	const { locals, fetch } = event;
	let world: World;
	try {
		world = await getWorld(locals.pb, id);
	} catch (err) {
		if (pbStatus(err) === 404) error(404, translate(locals.locale, 'error.notFound.title'));
		error(503, translate(locals.locale, 'error.unreachable'));
	}
	return {
		world,
		// A world shaped by ground this instance does not have cannot be
		// entered: the engine says so rather than showing another planet.
		fieldPath: await servedField(fetch, fieldId(world.recipe)),
		avatar: await visitorAvatar(event),
		name: visitorName(event),
		link: worldLink(world.id, locals.user !== null)
	};
}
