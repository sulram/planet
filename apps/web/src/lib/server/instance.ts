import type PocketBase from 'pocketbase';

/**
 * The instance: one record, what is the server's and not a world's, starting
 * with the front door. The server seeds the record; here it is
 * only read and, by an operator, pointed at a world.
 */
export interface Instance {
	id: string;
	/** The world `/` opens, or null while an operator has chosen none. */
	mainWorld: string | null;
}

export async function getInstance(pb: PocketBase): Promise<Instance> {
	const record = await pb.collection('instance').getFirstListItem('');
	return { id: record.id, mainWorld: String(record.main_world ?? '') || null };
}

export async function setMainWorld(pb: PocketBase, worldId: string): Promise<void> {
	const { id } = await getInstance(pb);
	await pb.collection('instance').update(id, { main_world: worldId });
}
