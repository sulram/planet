import { fail } from '@sveltejs/kit';
import { translate } from '$lib/i18n/config';
import { pageParam, pbStatus } from '$lib/server/pb';
import { listUsers, setOperator } from '$lib/server/users';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ locals, url }) => {
	try {
		return { users: await listUsers(locals.pb, pageParam(url)) };
	} catch {
		return { users: null };
	}
};

export const actions: Actions = {
	// Grants or removes the operator flag, with the operator's own token.
	// Nobody removes their own flag: the last operator cannot lock the door.
	operator: async ({ request, locals }) => {
		const form = await request.formData();
		const id = String(form.get('id') ?? '');
		const operator = form.get('operator') === 'true';
		if (!id) return fail(400, { error: translate(locals.locale, 'bo.users.error.update') });
		if (id === locals.user?.id && !operator) {
			return fail(400, { error: translate(locals.locale, 'bo.users.error.self') });
		}
		try {
			await setOperator(locals.pb, id, operator);
		} catch (err) {
			const key = pbStatus(err) === 0 ? 'error.unreachable' : 'bo.users.error.update';
			return fail(502, { error: translate(locals.locale, key) });
		}
		return { done: translate(locals.locale, operator ? 'bo.users.granted' : 'bo.users.removed') };
	}
};
