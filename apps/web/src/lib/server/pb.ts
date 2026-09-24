import PocketBase, { ClientResponseError, type ListResult, type RecordModel } from 'pocketbase';
import { env } from '$env/dynamic/private';

export const PB_URL = env.PB_URL || 'http://127.0.0.1:8090';
/** Where a browser reaches the same server: the world socket opens there. */
export const PB_PUBLIC_URL = env.PB_PUBLIC_URL || PB_URL;

/** Session data exposed to routes: a serializable subset of the `users` record. */
export type SessionUser = {
	id: string;
	email: string;
	name: string;
	operator: boolean;
	verified: boolean;
};

/**
 * One PocketBase client per request. The auth store is never shared between
 * requests, or one person's session would leak into another's.
 */
export function createServerClient(): PocketBase {
	const pb = new PocketBase(PB_URL);
	pb.autoCancellation(false);
	return pb;
}

/** HTTP status of a PocketBase failure; 0 when the server could not be reached. */
export function pbStatus(err: unknown): number {
	return err instanceof ClientResponseError ? err.status : 0;
}

export const PER_PAGE = 50;

export interface Paged<T> {
	items: T[];
	page: number;
	pages: number;
	total: number;
}

export function toPaged<T>(res: ListResult<RecordModel>, map: (r: RecordModel) => T): Paged<T> {
	return { items: res.items.map(map), page: res.page, pages: Math.max(1, res.totalPages), total: res.totalItems };
}

/** The `page` query parameter as a positive integer. */
export function pageParam(url: URL): number {
	const n = Number(url.searchParams.get('page'));
	return Number.isInteger(n) && n >= 1 ? n : 1;
}
