/**
 * Whether this is a touch screen: a phone or a tablet, held and touched.
 * Its primary pointer is coarse or cannot hover, and the screen takes
 * fingers; a laptop with a touch screen keeps its mouse and is not one.
 *
 * A touch screen walks a world: a stick, a drag that looks, buttons to jump
 * and fly. It offers no founding and no building, and its layout is the
 * design system's touch tokens (`tokens.css`).
 */
import { browser } from '$app/environment';
import { MediaQuery } from 'svelte/reactivity';

const coarse = new MediaQuery('pointer: coarse');
const hoverless = new MediaQuery('hover: none');

export const touch = {
	get current(): boolean {
		return browser && navigator.maxTouchPoints > 0 && (coarse.current || hoverless.current);
	}
};
