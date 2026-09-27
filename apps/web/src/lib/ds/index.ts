/**
 * Design system: the single import point.
 *
 *   import { Button, Field, Input } from '$lib/ds';
 *
 * Project rule: every reusable UI element lives here before a second screen
 * copies it. The living catalogue is at /ds.
 */
export { default as Alert } from './Alert.svelte';
export { default as Badge } from './Badge.svelte';
export { default as Button } from './Button.svelte';
export { default as Checkbox } from './Checkbox.svelte';
export { default as CodeInput } from './CodeInput.svelte';
export { default as Dialog } from './Dialog.svelte';
export { default as Field } from './Field.svelte';
export { default as Icon } from './Icon.svelte';
export type { IconName } from './Icon.svelte';
export { default as Input } from './Input.svelte';
export { default as LangSwitch } from './LangSwitch.svelte';
export { default as Page } from './Page.svelte';
export { default as PageHeader } from './PageHeader.svelte';
export type { Crumb } from './PageHeader.svelte';
export { default as Pager } from './Pager.svelte';
export { default as Panel } from './Panel.svelte';
export { default as Segmented } from './Segmented.svelte';
export { default as Select } from './Select.svelte';
export { default as Slider } from './Slider.svelte';
export { default as Spinner } from './Spinner.svelte';
export { default as Stack } from './Stack.svelte';
export { default as Stat } from './Stat.svelte';
export { default as Table } from './Table.svelte';
export { default as ThemeToggle } from './ThemeToggle.svelte';
export { default as Topbar } from './Topbar.svelte';
export { theme } from './theme.svelte';
export type { Theme } from './theme';
