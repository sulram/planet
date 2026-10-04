import type { WebPlugin } from '../plugin';
import Layer from './Layer.svelte';

/** Chat in the web front end: a bar at the foot of the world, and balloons over heads. */
const chat: WebPlugin = { name: 'chat', version: 1, label: 'engine.plugin.chat', Layer };

export default chat;
