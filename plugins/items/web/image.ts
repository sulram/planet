/**
 * A picture as it is placed: what the owner's client makes of a file before
 * the record names it. Three files go up, the original, a low version and a
 * thumb, each under its own hash, so a client far off fetches the small one
 * and a client near the original (ARCHITECTURE § Files). Their sides are the
 * engine's (`crates/client/src/pictures.rs`); `bun run docs` holds the two
 * to the same numbers. A picture that moves, a GIF, an animated WebP or a
 * video, goes up as it is, with its first frame as the two posters and how
 * long it plays in the record.
 */

/** The longest side of a thumb, and of a low version, in texels. */
export const THUMB_SIDE = 128;
export const LOW_SIDE = 1024;

/** The file kinds a picture is taken from: what the server sniffs. */
export const KINDS = ['image/png', 'image/jpeg', 'image/webp', 'image/gif', 'video/mp4'] as const;

/** How the smaller versions are kept: small, and read by every browser. */
const VERSION_KIND = 'image/webp';
const VERSION_QUALITY = 0.82;

/** A GIF frame with no delay, or one under what browsers honour, plays for this long, as in every browser. */
const SHORT_FRAME_US = 100_000;
const LEAST_FRAME_US = 20_000;

/** What a picture is at every distance, as the record names it. */
export interface Media {
	width: number;
	height: number;
	/** How long it plays, in milliseconds: 0 for a still. */
	millis: number;
	/** Asset names, `asset://<hash>.<ext>`. */
	thumb: string;
	low: string;
	full: string;
}

/** Sends a file to the world and resolves to its asset name (`$lib/files`). */
export type Upload = (file: Blob) => Promise<string>;

/**
 * A size no longer than `side` on its longest side, the aspect kept, never
 * grown: a picture smaller than a version is that version as it is.
 */
export function within(width: number, height: number, side: number): { width: number; height: number } {
	const longest = Math.max(width, height, 1);
	if (longest <= side) return { width, height };
	const scale = side / longest;
	return { width: Math.max(1, Math.round(width * scale)), height: Math.max(1, Math.round(height * scale)) };
}

/** Whether a file is one a picture is made of. */
export function isPicture(file: Blob): boolean {
	return (KINDS as readonly string[]).includes(file.type);
}

/** Whether a picture's file is a video: it plays through a video element, not an image decoder. */
export const isVideo = (file: Blob): boolean => file.type.startsWith('video/');

/**
 * How long a GIF or an animated WebP plays, in milliseconds, the sum of its
 * frames: 0 for a still, and 0 where the browser decodes no image frame by
 * frame, so the picture stands still everywhere.
 */
export function playsFor(count: number, durations: readonly (number | null)[]): number {
	if (count <= 1) return 0;
	const us = durations.reduce<number>((sum, frame) => sum + (frame === null || frame < LEAST_FRAME_US ? SHORT_FRAME_US : frame), 0);
	return Math.round(us / 1000);
}

/** The first frame of a picture and how long the whole plays. */
interface Poster {
	bitmap: ImageBitmap;
	millis: number;
}

/**
 * Makes the three files of a picture and sends them up: the original as it
 * is, and the two versions drawn down from its first frame with the
 * browser's own decoder.
 */
export async function prepareImage(file: Blob, upload: Upload): Promise<Media> {
	if (!isPicture(file)) throw new Error('kind');
	const { bitmap, millis } = isVideo(file) ? await videoPoster(file) : await imagePoster(file);
	try {
		const { width, height } = bitmap;
		const [full, low, thumb] = await Promise.all([
			upload(file),
			upload(await version(bitmap, LOW_SIDE)),
			upload(await version(bitmap, THUMB_SIDE))
		]);
		return { width, height, millis, thumb, low, full };
	} finally {
		bitmap.close();
	}
}

/** An image's first frame, and the sum of its frames where it has more than one. */
async function imagePoster(file: Blob): Promise<Poster> {
	const bitmap = await createImageBitmap(file);
	if (!('ImageDecoder' in globalThis)) return { bitmap, millis: 0 };
	const decoder = new ImageDecoder({ data: await file.arrayBuffer(), type: file.type });
	try {
		// The track, and its frame count, exist only once the track list is ready.
		await decoder.tracks.ready;
		await decoder.completed;
		const count = decoder.tracks.selectedTrack?.frameCount ?? 1;
		const durations: (number | null)[] = [];
		for (let index = 0; index < count && count > 1; index++) {
			const { image } = await decoder.decode({ frameIndex: index });
			durations.push(image.duration);
			image.close();
		}
		return { bitmap, millis: playsFor(count, durations) };
	} finally {
		decoder.close();
	}
}

/** A video's first frame, off a video element loaded from the file, and its length. */
async function videoPoster(file: Blob): Promise<Poster> {
	const url = URL.createObjectURL(file);
	const video = document.createElement('video');
	video.muted = true;
	video.playsInline = true;
	video.preload = 'auto';
	try {
		await new Promise<void>((loaded, failed) => {
			video.onloadeddata = () => loaded();
			video.onerror = () => failed(new Error('canvas'));
			video.src = url;
		});
		const bitmap = await createImageBitmap(video);
		return { bitmap, millis: Math.max(1, Math.round(video.duration * 1000)) };
	} finally {
		video.removeAttribute('src');
		video.load();
		URL.revokeObjectURL(url);
	}
}

/** The picture drawn down to a version no longer than `side`, as WebP. */
async function version(bitmap: ImageBitmap, side: number): Promise<Blob> {
	const { width, height } = within(bitmap.width, bitmap.height, side);
	const canvas = new OffscreenCanvas(width, height);
	const context = canvas.getContext('2d');
	if (!context) throw new Error('canvas');
	context.imageSmoothingQuality = 'high';
	context.drawImage(bitmap, 0, 0, width, height);
	return canvas.convertToBlob({ type: VERSION_KIND, quality: VERSION_QUALITY });
}
