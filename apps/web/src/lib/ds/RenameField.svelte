<script lang="ts">
	// The input half of a rename in place. It exists only while editing: the
	// caller owns the trigger and the resting view, which differ from place
	// to place (a panel's title, a row of a list), while the keyboard
	// contract does not. Esc cancels before the blur commits, so the flag
	// crosses both events; Enter blurs; an unchanged name is no edit.
	interface Props {
		value: string;
		/** What the field renames, for the screen reader. */
		label: string;
		onCommit: (name: string) => void;
		/** Editing ended, whichever way: the caller closes its own field. */
		onDone: () => void;
		placeholder?: string;
		maxlength?: number;
	}

	let { value, label, onCommit, onDone, placeholder, maxlength = 64 }: Props = $props();

	let cancelled = false;

	function focusField(node: HTMLInputElement) {
		node.focus();
		node.select();
	}

	function keydown(event: KeyboardEvent) {
		const field = event.currentTarget as HTMLInputElement;
		if (event.key === 'Enter') field.blur();
		if (event.key === 'Escape') {
			// The page hears Esc too (closing the panel): leaving a rename is no reason to leave the panel.
			event.stopPropagation();
			cancelled = true;
			field.blur();
		}
	}

	function blur(event: FocusEvent) {
		const field = event.currentTarget as HTMLInputElement;
		const wasCancelled = cancelled;
		cancelled = false;
		onDone();
		if (!wasCancelled && field.value.trim() !== value) onCommit(field.value);
	}
</script>

<input class="rename" {value} {placeholder} {maxlength} aria-label={label} use:focusField onblur={blur} onkeydown={keydown} />

<style>
	.rename {
		width: 100%;
		min-width: 0;
		padding: var(--sp-1) var(--sp-2);
		border: 1px solid var(--border);
		border-radius: var(--radius-xs, 2px);
		background: var(--bg-sunken, var(--bg));
		color: var(--fg);
		/* The field takes the size and weight of the text it replaces, so it does not shove the layout about while one types. */
		font: inherit;
	}
	.rename:focus {
		border-color: var(--fg);
		outline: none;
	}
</style>
