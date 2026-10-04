import type { WebPlugin } from '$lib/plugins/plugin';
import Layer from './Layer.svelte';
import { t } from './words';

/** Chat in the web front end: a bar at the foot of the world, and balloons over heads. */
const chat: WebPlugin = {
	name: 'chat',
	label: () => t('name'),
	Layer,
	hints: [{ keys: 'Enter', does: () => t('hint.chat') }]
};

export default chat;
