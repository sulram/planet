import { redirect } from '@sveltejs/kit';
import { HOME } from '$lib/server/auth';
import type { RequestHandler } from './$types';

// Ends the session: clear the auth store; hooks.server.ts writes the empty cookie.
export const POST: RequestHandler = ({ locals }) => {
	locals.pb.authStore.clear();
	redirect(303, HOME);
};
