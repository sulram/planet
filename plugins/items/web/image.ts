/**
 * A picture as it is placed: what the owner's client makes of a file before
 * the record names it. Three files go up, the original, a low version and a
 * thumb, each under its own hash, so a client far off fetches the small one
 * and a client near the original (ARCHITECTURE § Files). Their sides are the
 * engine's (`crates/client/src/pictures.rs`); `bun run docs` holds the two
 * to the same numbers.
 */

/** The longest side of a thumb, and of a low version, in texels. */
export const THUMB_SIDE = 128;
export const LOW_SIDE = 1024;

/** The file kinds a picture is taken from: what the server sniffs. */
export const KINDS = ['image/png', 'image/jpeg', 'image/webp'] as const;

/** How the smaller versions are kept: small, and read by every browser. */
const VERSION_KIND = 'image/webp';
const VERSION_QUALITY = 0.82;

/** What a picture is at every distance, as the record names it. */
export interface Media {
	width: number;
	height: number;
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

/**
 * Makes the three files of a picture and sends them up: the original as it
 * is, and the two versions drawn down with the browser's own decoder.
 */
export async function prepareImage(file: Blob, upload: Upload): Promise<Media> {
	if (!isPicture(file)) throw new Error('kind');
	const bitmap = await createImageBitmap(file);
	try {
		const { width, height } = bitmap;
		const [full, low, thumb] = await Promise.all([
			upload(file),
			upload(await version(bitmap, LOW_SIDE)),
			upload(await version(bitmap, THUMB_SIDE))
		]);
		return { width, height, thumb, low, full };
	} finally {
		bitmap.close();
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
