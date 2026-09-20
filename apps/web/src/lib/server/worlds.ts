import type PocketBase from 'pocketbase';
import type { RecordModel } from 'pocketbase';
import type { Recipe, World } from '$lib/world';
import { PER_PAGE, toPaged, type Paged } from './pb';

/** A world as the backoffice lists it: with the owner's email when visible. */
export interface WorldRow extends World {
	ownerEmail: string;
}

function isObject(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function toWorld(r: RecordModel): World {
	return {
		id: r.id,
		name: String(r.name ?? ''),
		recipe: {
			seed: String(r.seed ?? ''),
			generator_version: Number(r.generator_version ?? 0),
			params: isObject(r.params) ? r.params : {}
		},
		owner: String(r.owner ?? ''),
		created: String(r.created ?? ''),
		updated: String(r.updated ?? '')
	};
}

export async function listWorlds(pb: PocketBase, page = 1): Promise<Paged<World>> {
	return toPaged(await pb.collection('worlds').getList(page, PER_PAGE, { sort: '-created' }), toWorld);
}

export async function listWorldRows(pb: PocketBase, page = 1): Promise<Paged<WorldRow>> {
	const res = await pb.collection('worlds').getList(page, PER_PAGE, { sort: '-created', expand: 'owner' });
	return toPaged(res, (r) => ({ ...toWorld(r), ownerEmail: String(r.expand?.owner?.email ?? '') }));
}

export async function getWorld(pb: PocketBase, id: string): Promise<World> {
	return toWorld(await pb.collection('worlds').getOne(id));
}

/** "Create world" writes one row: the recipe, a name and the owner. */
export async function createWorld(pb: PocketBase, owner: string, name: string, recipe: Recipe): Promise<World> {
	return toWorld(await pb.collection('worlds').create({ name, owner, ...recipe }));
}

/** The recipe is immutable; the name is the only thing that changes. */
export async function renameWorld(pb: PocketBase, id: string, name: string): Promise<void> {
	await pb.collection('worlds').update(id, { name });
}

export async function deleteWorld(pb: PocketBase, id: string): Promise<void> {
	await pb.collection('worlds').delete(id);
}
