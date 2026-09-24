import { enterWorld } from '$lib/server/enter';
import type { PageServerLoad } from './$types';

// Any world, by link. Unlisted, never locked (DECISIONS 70).
export const load: PageServerLoad = (event) => enterWorld(event, event.params.id);
