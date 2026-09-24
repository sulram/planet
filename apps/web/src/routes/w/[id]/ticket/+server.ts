import { error, json } from '@sveltejs/kit';
import { mintTicket } from '$lib/server/session';
import type { RequestHandler } from './$types';

// A fresh ticket for the world's socket, with the person's own token. The
// engine asks for one before every connection, so a reconnect is never a
// visitor by accident.
export const POST: RequestHandler = async ({ locals }) => {
	if (!locals.user) error(401);
	try {
		return json({ ticket: await mintTicket(locals.pb) });
	} catch {
		error(503);
	}
};
