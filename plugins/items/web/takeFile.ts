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
import { isPicture, isVideo, prepareImage, type Media } from './image';
import type { Picked } from './protocol';

/** What a file is to items, by its name and type: a picture, a model, a sound, or none of them. */
export type Given = 'picture' | 'model' | 'sound';

/** The audio kinds the world takes: what every browser plays. */
const SOUND_TYPES = ['audio/mpeg', 'audio/mp4', 'audio/x-m4a', 'audio/ogg'];

/** Why a file was not taken: the world's codes, and the browser's. */
export type NotTaken = 'kind' | 'size' | 'level' | 'key' | 'unreachable' | 'canvas';
const refusals: readonly NotTaken[] = ['kind', 'size', 'level', 'key', 'unreachable', 'canvas'];

/** What a file would be, by its name and type, before it is read. Null for one items takes no interest in. */
export function givenOf(name: string, type: string): Given | null {
	if (/\.vox$/i.test(name)) return 'model';
	if (isPicture({ type } as Blob) || /\.(png|jpe?g|webp|gif|mp4)$/i.test(name)) return 'picture';
	if (SOUND_TYPES.includes(type) || /\.(mp3|m4a|ogg|oga|opus)$/i.test(name)) return 'sound';
	return null;
}

/** What a file being dragged over the page would be, from the types a drag says before the drop: a picture, still or moving, is known by its type, a `.vox` says nothing until dropped. */
export function draggedOf(types: readonly string[]): Given | 'any' {
	if (types.some((type) => type.startsWith('image/') || type.startsWith('video/'))) return 'picture';
	if (types.some((type) => type.startsWith('audio/'))) return 'sound';
	return 'any';
}

/** Reads the refusal code of what was thrown, in the words this panel has. */
export function notTaken(error: unknown): NotTaken {
	const code = error instanceof Error ? error.message : '';
	return refusals.find((known) => known === code) ?? 'unreachable';
}

/** How long an audio file plays, in milliseconds, off an audio element loaded from it; 0 when the browser cannot say. */
async function playsFor(file: Blob): Promise<number> {
	const url = URL.createObjectURL(file);
	const audio = new Audio();
	audio.preload = 'metadata';
	try {
		await new Promise<void>((loaded, failed) => {
			audio.onloadedmetadata = () => loaded();
			audio.onerror = () => failed(new Error('kind'));
			audio.src = url;
		});
		return Number.isFinite(audio.duration) ? Math.round(audio.duration * 1000) : 0;
	} finally {
		audio.removeAttribute('src');
		audio.load();
		URL.revokeObjectURL(url);
	}
}

/** What taking a file came to: the item it is, and a picture's record. */
export interface Taken {
	given: Given;
	media?: Media;
}

/**
 * Takes a file: sends it to the world and places the item it is in front
 * of the body, as soon as it is ready, or puts it into the picture picked.
 * Throws an `Error` whose message is a `NotTaken` code.
 */
export async function takeFile(seam: Seam, file: File, picked?: Picked | null): Promise<Taken> {
	const given = givenOf(file.name, file.type);
	if (given === null) throw new Refused('kind');
	if (given === 'model') {
		const head = new Uint8Array(await file.slice(0, 16).arrayBuffer());
		if (kindOf(head) !== 'vox') throw new Refused('kind');
		const asset = await seam.upload(file);
		seam.command('take', { item: 'model', asset, ahead: true });
		return { given };
	}
	if (given === 'sound') {
		const head = new Uint8Array(await file.slice(0, 16).arrayBuffer());
		if (!['mp3', 'm4a', 'ogg'].includes(kindOf(head) ?? '')) throw new Refused('kind');
		const [asset, millis] = await Promise.all([seam.upload(file), playsFor(file)]);
		seam.command('take', { item: 'sound', clip: { file: asset, millis }, ahead: true });
		return { given };
	}
	const media = await prepareImage(file, (blob) => seam.upload(blob));
	// A picture picked takes either kind of file in its place: a still and a
	// video are the same part, told apart by their file.
	if (picked?.item === 'picture' || picked?.item === 'video') {
		seam.command('set', { media });
	} else {
		seam.command('take', { item: isVideo(file) ? 'video' : 'picture', media, ahead: true });
	}
	return { given, media };
}
