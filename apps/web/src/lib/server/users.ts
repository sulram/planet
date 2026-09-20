import type PocketBase from 'pocketbase';
import type { RecordModel } from 'pocketbase';
import { PER_PAGE, toPaged, type Paged } from './pb';

/** A user as the backoffice lists it. Read with the operator's own token. */
export interface UserRow {
	id: string;
	email: string;
	name: string;
	verified: boolean;
	operator: boolean;
	created: string;
}

function toRow(r: RecordModel): UserRow {
	return {
		id: r.id,
		email: String(r.email ?? ''),
		name: String(r.name ?? ''),
		verified: r.verified === true,
		operator: r.operator === true,
		created: String(r.created ?? '')
	};
}

export async function listUsers(pb: PocketBase, page = 1): Promise<Paged<UserRow>> {
	return toPaged(await pb.collection('users').getList(page, PER_PAGE, { sort: '-created' }), toRow);
}

export async function setOperator(pb: PocketBase, id: string, operator: boolean): Promise<void> {
	await pb.collection('users').update(id, { operator });
}
