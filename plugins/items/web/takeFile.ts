/**
 * A file given to items, dropped anywhere on the page or chosen in the
 * panel, becomes the item it is: a picture, or a model from a `.vox`. It is
 * sent to the world through the seam and taken in hand, so the next click
 * places it; with a picture picked in the world, a picture given takes that
 * picture's place instead. Pure of the DOM: it reads the file alone.
 */
import { kindOf } from '$lib/files';
import { Refused } from '$lib/instance';
import type { Seam } from '$lib/plugins/plugin';
import { isPicture, prepareImage, type Media } from './image';
import type { Picked } from './protocol';

/** What a file is to items, by its name and type: a picture, a model, or neither. */
export type Given = 'picture' | 'model';

/** Why a file was not taken: the world's codes, and the browser's. */
export type NotTaken = 'kind' | 'size' | 'level' | 'key' | 'unreachable' | 'canvas';
const refusals: readonly NotTaken[] = ['kind', 'size', 'level', 'key', 'unreachable', 'canvas'];

/** What a file would be, by its name and type, before it is read. Null for one items takes no interest in. */
export function givenOf(name: string, type: string): Given | null {
	if (/\.vox$/i.test(name)) return 'model';
	if (isPicture({ type } as Blob) || /\.(png|jpe?g|webp)$/i.test(name)) return 'picture';
	return null;
}

/** What a file being dragged over the page would be, from the types a drag says before the drop: a picture is known by its type, a `.vox` says nothing until dropped. */
export function draggedOf(types: readonly string[]): Given | 'any' {
	return types.some((type) => type.startsWith('image/')) ? 'picture' : 'any';
}

/** Reads the refusal code of what was thrown, in the words this panel has. */
export function notTaken(error: unknown): NotTaken {
	const code = error instanceof Error ? error.message : '';
	return refusals.find((known) => known === code) ?? 'unreachable';
}

/** What taking a file came to: the item it is, and a picture's record. */
export interface Taken {
	given: Given;
	media?: Media;
}

/**
 * Takes a file: sends it to the world and puts the item it is in hand, or
 * into the picture picked. Throws an `Error` whose message is a `NotTaken`
 * code.
 */
export async function takeFile(seam: Seam, file: File, picked?: Picked | null): Promise<Taken> {
	const given = givenOf(file.name, file.type);
	if (given === null) throw new Refused('kind');
	if (given === 'model') {
		const head = new Uint8Array(await file.slice(0, 16).arrayBuffer());
		if (kindOf(head) !== 'vox') throw new Refused('kind');
		const asset = await seam.upload(file);
		seam.command('take', { item: 'model', asset });
		return { given };
	}
	const media = await prepareImage(file, (blob) => seam.upload(blob));
	if (picked?.item === 'picture') {
		seam.command('set', { media });
	} else {
		seam.command('take', { item: 'picture', media });
	}
	return { given, media };
}
