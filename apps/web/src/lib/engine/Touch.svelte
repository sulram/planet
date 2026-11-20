<script lang="ts">
	import { Icon } from '$lib/ds';
	import { t } from '$lib/i18n';
	import type { Mode, Stick } from './index';

	// What a touch screen has in place of the keys. The stick is the web
	// shell's: it reads the fingers and says where the stick is, and this
	// draws it. The buttons press the keys a keyboard would, through the
	// engine's own key path: up held jumps on foot and rises in flight, down
	// held sinks in flight, the plane takes flight or lands, and the eye
	// swings the camera into the head and out. They sit in the bottom right
	// corner, where the right thumb rests; the left half of the world is the
	// stick's.
	interface Props {
		stick: Stick | null;
		mode: Mode;
		/** Presses a key or lets it go, by the name the web gives it. */
		onkey: (code: string, down: boolean) => void;
	}

	let { stick, mode, onkey }: Props = $props();

	/**
	 * Holds a key while the button is held. The press keeps the focus where
	 * it was, so the canvas holds on to the finger on the stick, and the
	 * button keeps its finger wherever it slides.
	 */
	function hold(code: string) {
		return {
			onpointerdown(event: PointerEvent) {
				event.preventDefault();
				(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
				onkey(code, true);
			},
			onpointerup: () => onkey(code, false),
			onpointercancel: () => onkey(code, false),
			oncontextmenu: (event: Event) => event.preventDefault()
		};
	}

	/** Presses a key once, as a tap. */
	function tap(code: string) {
		return {
			onpointerdown: (event: PointerEvent) => event.preventDefault(),
			onclick() {
				onkey(code, true);
				onkey(code, false);
			}
		};
	}
</script>

{#if stick}
	<div class="stick" aria-hidden="true">
		<span class="reach" style:left="{stick.center[0]}px" style:top="{stick.center[1]}px" style:--reach="{stick.reach}px"></span>
		<span class="knob" style:left="{stick.at[0]}px" style:top="{stick.at[1]}px"></span>
	</div>
{/if}

<div class="thumbs" role="group" aria-label={t('engine.touch')}>
	<button type="button" class="eye" aria-label={t('engine.touch.eye')} {...tap('KeyP')}>
		<Icon name="eye" />
	</button>
	<button type="button" class="fly" aria-label={t('engine.touch.fly')} aria-pressed={mode === 'fly'} {...tap('KeyF')}>
		<Icon name="plane" />
	</button>
	{#if mode === 'fly'}
		<button type="button" class="down" aria-label={t('engine.touch.down')} {...hold('KeyC')}>
			<Icon name="chevrons-down" />
		</button>
	{/if}
	<button type="button" class="up" aria-label={t('engine.touch.up')} {...hold('Space')}>
		<Icon name="chevrons-up" />
	</button>
</div>

<style>
	/* The stick draws over the world and takes nothing: the finger is the
	   canvas's. Round, since the reach is the same every way. */
	.stick {
		position: absolute;
		inset: 0;
		z-index: var(--z-panel);
		pointer-events: none;
	}
	.reach,
	.knob {
		position: absolute;
		border-radius: 50%;
		transform: translate(-50%, -50%);
	}
	.reach {
		width: calc(2 * var(--reach));
		height: calc(2 * var(--reach));
		border: var(--bw) solid var(--on-world);
		background: color-mix(in srgb, var(--on-world-shadow) 25%, transparent);
	}
	.knob {
		width: var(--control-h);
		height: var(--control-h);
		background: var(--on-world);
		box-shadow: 0 0 var(--sp-1) var(--on-world-shadow);
	}
	/* Up at the corner, under the thumb; the plane over it, down beside it
	   in flight, the eye, pressed least, farthest. Above the foot of the
	   world, where a plugin's bar sits. */
	.thumbs {
		position: absolute;
		right: var(--sp-4);
		bottom: calc(2 * var(--sp-5) + var(--control-h));
		z-index: var(--z-panel);
		display: grid;
		grid-template: 'eye fly' auto 'down up' auto / auto auto;
		justify-items: end;
		gap: var(--sp-4);
	}
	button {
		display: grid;
		place-items: center;
		width: var(--thumb-h);
		height: var(--thumb-h);
		border: var(--bw) solid var(--border-strong);
		background: var(--bg-overlay);
		color: var(--text);
		font-size: calc(2 * var(--fs));
		touch-action: none;
		user-select: none;
		-webkit-user-select: none;
		-webkit-touch-callout: none;
	}
	button:active,
	button[aria-pressed='true'] {
		background: var(--action);
		color: var(--action-text);
	}
	.eye {
		grid-area: eye;
	}
	.fly {
		grid-area: fly;
	}
	.down {
		grid-area: down;
	}
	.up {
		grid-area: up;
	}
</style>
