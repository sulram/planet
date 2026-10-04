/**
 * A plugin's half in the web front end, and what the stage offers it: a layer
 * over the world, the core's readings as props, and the seam under the
 * plugin's own name (docs/PLUGINS.md).
 */
import type { Component } from 'svelte';
import type { Anchor, Command, Level, PeerInfo } from '$lib/engine';
import type { MessageKey } from '$lib/i18n';

/** The seam, as one plugin reaches it. */
export interface Seam {
	/** Sends a command of this plugin: `say` goes out as `chat.say`. */
	command(kind: string, body?: Record<string, unknown>): void;
	/** Sends one of the core's own commands: go to a place, take a mode. */
	core(command: Command): void;
	/**
	 * Hears this plugin's events, each under its name without the plugin's:
	 * `said` for `chat.said`. Returns how to stop hearing.
	 */
	listen(hear: (kind: string, event: Record<string, unknown>) => void): () => void;
	/** Lets the world go, so a form in the layer has the keys and the pointer. */
	release(): void;
	/** Hands the world back. */
	take(): void;
}

/** What a plugin's layer is mounted with, while its plugin is on in this world. */
export interface LayerProps {
	seam: Seam;
	/** Whether the link to the world is open. */
	online: boolean;
	/** This client's own session, while online. */
	me: number | null;
	/** What the world said this person may do. */
	level: Level | null;
	/** The paints a cell can take, as `#rrggbb`, in order: the world's palette. */
	palette: string[];
	peers: PeerInfo[];
	/** Where every head in view is on the screen, each frame. */
	anchors: Anchor[];
}

/** A key of a plugin, as the help panel lists it while the plugin is on. */
export interface Hint {
	/** What is pressed: said as it is, `B`, or by a message where words name it. */
	keys: string | { message: MessageKey };
	does: MessageKey;
}

/** A plugin as the web front end holds it. */
export interface WebPlugin {
	/** The name the world's statement says. */
	name: string;
	/** The version of its seam. A world that speaks another leaves it unmounted. */
	version: number;
	/** What it is called where an admin switches it. */
	label: MessageKey;
	/** What it draws over the world. A plugin with nothing to show has none. */
	Layer?: Component<LayerProps>;
	/** The keys it asks for, for the help panel. */
	hints?: readonly Hint[];
}
