import { fail } from '@sveltejs/kit';
import { translate } from '$lib/i18n/config';
import { pageParam, pbStatus } from '$lib/server/pb';
import { getInstance, setMainWorld } from '$lib/server/instance';
import { deleteWorld, listWorldRows, renameWorld } from '$lib/server/worlds';
import { WORLD_NAME_MAX } from '$lib/world';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ locals, url }) => {
	try {
		const [worlds, instance] = await Promise.all([listWorldRows(locals.pb, pageParam(url)), getInstance(locals.pb)]);
		return { worlds, mainWorld: instance.mainWorld };
	} catch {
		return { worlds: null, mainWorld: null };
	}
};

// Every write runs with the operator's own token; PocketBase rules decide.
export const actions: Actions = {
	rename: async ({ request, locals }) => {
		const form = await request.formData();
		const id = String(form.get('id') ?? '');
		const name = String(form.get('name') ?? '').trim();
		if (!id || !name || name.length > WORLD_NAME_MAX) {
			return fail(400, { error: translate(locals.locale, 'bo.worlds.error.name', { max: WORLD_NAME_MAX }) });
		}
		try {
			await renameWorld(locals.pb, id, name);
		} catch (err) {
			const key = pbStatus(err) === 0 ? 'error.unreachable' : 'bo.worlds.error.rename';
			return fail(502, { error: translate(locals.locale, key) });
		}
		return { done: translate(locals.locale, 'bo.worlds.renamed', { name }) };
	},

	// The front door (DECISIONS 70): the world `/` opens.
	main: async ({ request, locals }) => {
		const form = await request.formData();
		const id = String(form.get('id') ?? '');
		const name = String(form.get('name') ?? '');
		try {
			await setMainWorld(locals.pb, id);
		} catch (err) {
			const key = pbStatus(err) === 0 ? 'error.unreachable' : 'bo.worlds.error.main';
			return fail(502, { error: translate(locals.locale, key) });
		}
		return { done: translate(locals.locale, 'bo.worlds.mainSet', { name }) };
	},

	delete: async ({ request, locals }) => {
		const form = await request.formData();
		const id = String(form.get('id') ?? '');
		try {
			await deleteWorld(locals.pb, id);
		} catch (err) {
			const key = pbStatus(err) === 0 ? 'error.unreachable' : 'bo.worlds.error.delete';
			return fail(502, { error: translate(locals.locale, key) });
		}
		return { done: translate(locals.locale, 'bo.worlds.deleted') };
	}
};
