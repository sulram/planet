import type { LayoutServerLoad } from './$types';

// Exposes the session user and the locale to every route. Both come from
// `locals`, filled in hooks.server.ts.
export const load: LayoutServerLoad = ({ locals }) => {
	return { user: locals.user, locale: locals.locale };
};
