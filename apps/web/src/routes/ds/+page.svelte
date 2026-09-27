<script lang="ts">
	import {
		Alert,
		Badge,
		Button,
		Checkbox,
		CodeInput,
		Dialog,
		Field,
		Icon,
		Input,
		LangSwitch,
		Page,
		PageHeader,
		Pager,
		Panel,
		Segmented,
		Select,
		Slider,
		Spinner,
		Stack,
		Stat,
		Table,
		ThemeToggle,
		Topbar
	} from '$lib/ds';

	// The living catalogue. A developer reference, written in the project
	// language like the code it documents; it is not product UI, so its prose
	// does not go through i18n. Components render with their real strings.

	let shadows = $state(true);
	let settled = $state(0.5);
	let settledAt = $state(0.5);
	let cover = $state(0.5);
	let curve = $state('aces');
	const curves = [
		{ value: 'aces', label: 'ACES' },
		{ value: 'agx', label: 'AgX' }
	];

	const primitives = ['black', 'grey-900', 'grey-800', 'grey-600', 'grey-400', 'grey-300', 'grey-100', 'white', 'red-600', 'red-400'];
	const semantics = [
		'bg',
		'bg-inset',
		'bg-overlay',
		'text',
		'text-muted',
		'border',
		'border-strong',
		'action',
		'action-text',
		'focus',
		'danger'
	];
	const spaces = [1, 2, 3, 4, 5, 6, 7, 8];
	const speeds = [
		{ value: 'walk', label: 'Walk' },
		{ value: 'fly', label: 'Fly' }
	] as const;

	let speed = $state<(typeof speeds)[number]['value']>('walk');
	let email = $state('');
	let code = $state('3635');
	let loading = $state(false);
	let confirming = $state(false);
	let deleting = $state(false);
	let outcome = $state('');

	function wait(ms: number): Promise<void> {
		return new Promise((resolve) => setTimeout(resolve, ms));
	}

	async function pretend() {
		loading = true;
		await wait(1200);
		loading = false;
	}
</script>

<svelte:head>
	<title>Design system · planet</title>
</svelte:head>

<Topbar />

<Page>
	<PageHeader
		title="Design system"
		lede="Black and white, one typeface at one size, zero radius. Hierarchy comes from weight, case, spacing and borders. Every component below is the real one, in its states."
		trail={[{ label: 'planet', href: '/' }]}
	>
		{#snippet actions()}
			<LangSwitch />
			<ThemeToggle />
		{/snippet}
	</PageHeader>

	<div class="sections">
		<section>
			<h2>Rules</h2>
			<ul class="rules">
				<li>Components reference semantic tokens only. Primitives appear in <code>tokens.css</code> and nowhere else.</li>
				<li>One font size: <code>--fs</code> (10px). Never scale type; use weight, upper case, letter spacing, borders.</li>
				<li>One radius: <code>--radius</code> (0). Borders are <code>--bw</code> wide.</li>
				<li>Theme: <code>light-dark()</code> driven by <code>color-scheme</code>. No choice follows the OS; the <code>theme</code> cookie pins it and the server stamps <code>&lt;html data-theme&gt;</code>, so nothing flashes.</li>
				<li>
					Danger is the one colour, a single restrained red. An error or a destructive action must never read as ordinary
					chrome. Colour is never the only carrier: alerts and field errors lead with a <code>!</code> marker, destructive
					buttons say what they destroy and always confirm in a Dialog.
				</li>
				<li>Every wait gives a signal: a Button that waits for the server takes <code>loading</code>.</li>
				<li>No user visible literal in a component: every string goes through <code>t()</code>.</li>
			</ul>
		</section>

		<section>
			<h2>Colour: primitives</h2>
			<div class="swatches">
				{#each primitives as name (name)}
					<div class="swatch"><span style:background="var(--{name})"></span><code>--{name}</code></div>
				{/each}
			</div>
		</section>

		<section>
			<h2>Colour: semantics</h2>
			<div class="swatches">
				{#each semantics as name (name)}
					<div class="swatch"><span style:background="var(--{name})"></span><code>--{name}</code></div>
				{/each}
			</div>
		</section>

		<section>
			<h2>Type</h2>
			<Stack>
				<p class="caps">Title: bold, upper case, spaced</p>
				<p class="strong">Emphasis: bold</p>
				<p>Body: regular. The quick brown fox jumps over the lazy dog. 0123456789</p>
				<p class="muted">Muted: secondary information</p>
			</Stack>
		</section>

		<section>
			<h2>Space</h2>
			<div class="spaces">
				{#each spaces as n (n)}
					<div class="space"><span style:width="var(--sp-{n})"></span><code>--sp-{n}</code></div>
				{/each}
			</div>
		</section>

		<section>
			<h2>Button</h2>
			<div class="row">
				<Button>Primary</Button>
				<Button variant="ghost">Ghost</Button>
				<Button variant="danger">Danger</Button>
				<Button href="/ds">Link</Button>
			</div>
			<div class="row">
				<Button disabled>Primary</Button>
				<Button variant="ghost" disabled>Ghost</Button>
				<Button variant="danger" disabled>Danger</Button>
			</div>
			<div class="row">
				<Button loading>Primary</Button>
				<Button variant="ghost" loading>Ghost</Button>
				<Button variant="danger" loading>Danger</Button>
				<Button variant="ghost" {loading} onclick={pretend}>Click to wait</Button>
			</div>
		</section>

		<section>
			<h2>Field and Input</h2>
			<div class="form">
				<Stack>
					<Field label="Email" for="ds-email" hint="A hint explains what is expected.">
						<Input id="ds-email" type="email" placeholder="you@example.com" bind:value={email} />
					</Field>
					<Field label="Shape" hint="Without `for`, the label names a group instead of one control.">
						<Segmented
							options={[
								{ value: 'a', label: 'Generated' },
								{ value: 'b', label: 'Earth' }
							]}
							value="a"
							label="Shape"
						/>
					</Field>
					<Field label="Code" for="ds-code" hint="One cell per digit, in two halves, as the email spells it. Paste it with the space.">
						<CodeInput id="ds-code" bind:value={code} />
					</Field>
					<Field label="Refused code" for="ds-code-refused" error="This code is wrong or expired.">
						<CodeInput id="ds-code-refused" value="12345678" invalid />
					</Field>
					<Field label="Seed" for="ds-seed" error="An error replaces the hint and is shown whole.">
						<Input id="ds-seed" value="not a seed" invalid />
					</Field>
					<Field label="Read only" for="ds-readonly">
						<Input id="ds-readonly" value="00000000deadbeef" readonly />
					</Field>
					<Field label="Disabled" for="ds-disabled">
						<Input id="ds-disabled" value="Unavailable" disabled />
					</Field>
				</Stack>
			</div>
		</section>

		<section>
			<h2>Segmented</h2>
			<div class="row">
				<Segmented options={speeds} value={speed} label="Movement" onselect={(value) => (speed = value)} />
				<span class="muted">Selected: {speed}</span>
			</div>
		</section>

		<section>
			<h2>Checkbox, Slider and Select</h2>
			<div class="row">
				<Checkbox checked={shadows} onchange={(value) => (shadows = value)}>Shadows</Checkbox>
				<Checkbox checked={false} disabled>Unavailable</Checkbox>
			</div>
			<Stack>
				<Slider label="Cover" value={cover} min={0} max={1} oninput={(value) => (cover = value)} />
				<Slider
					label="Settled"
					value={settled}
					min={0}
					max={1}
					oninput={(value) => (settled = value)}
					onchange={(value) => (settledAt = value)}
				/>
				<p class="note">`onchange` fires once, when the drag ends: last let go at {settledAt.toFixed(2)}.</p>
				<Slider label="Disabled" value={0.3} min={0} max={1} disabled />
				<Select label="Tone map" options={curves} value={curve} onselect={(value) => (curve = value)} />
			</Stack>
		</section>

		<section>
			<h2>Icon</h2>
			<p class="muted">A stroke in the current colour at the text size. Lucide's paths, copied one at a time: the set is what the product uses.</p>
			<div class="row">
				<Icon name="users" label="Near" />
				<Icon name="globe" label="World" />
				<Icon name="send" label="Send" />
				<Icon name="map-pin" label="Place" />
				<Icon name="pencil" label="Edit" />
				<Button variant="ghost"><Icon name="send" /> With a label</Button>
			</div>
		</section>

		<section>
			<h2>Alert</h2>
			<Stack>
				<Alert>Information that does not block anything.</Alert>
				<Alert title="With a title">The title names the state; the body says what to do next.</Alert>
				<Alert variant="danger" title="Something failed">The message appears whole, with the way out.</Alert>
			</Stack>
		</section>

		<section>
			<h2>Badge and Spinner</h2>
			<div class="row">
				<Badge>Outline</Badge>
				<Badge variant="solid">Solid</Badge>
				<Spinner />
			</div>
		</section>

		<section>
			<h2>Dialog</h2>
			<div class="row">
				<Button variant="ghost" onclick={() => (confirming = true)}>Confirm</Button>
				<Button variant="danger" onclick={() => (deleting = true)}>Delete</Button>
				{#if outcome}<span class="muted">{outcome}</span>{/if}
			</div>
			<Dialog
				bind:open={confirming}
				title="Make operator"
				confirmLabel="Make operator"
				onconfirm={async () => {
					await wait(800);
					outcome = 'Confirmed.';
				}}
			>
				<p>A plain confirmation. The confirm button shows the wait.</p>
			</Dialog>
			<Dialog
				bind:open={deleting}
				title="Delete world"
				confirmLabel="Delete"
				danger
				onconfirm={async () => {
					await wait(800);
					outcome = 'Deleted.';
				}}
			>
				<p>Destructive actions always confirm here. With <code>action</code> the box posts to a form action instead.</p>
			</Dialog>
		</section>

		<section>
			<h2>Table and Pager</h2>
			<Table>
				<table>
					<thead>
						<tr><th>Name</th><th>Seed</th><th>Operator</th><th></th></tr>
					</thead>
					<tbody>
						<tr>
							<td>First light</td>
							<td class="muted">00000000deadbeef</td>
							<td><Badge variant="solid">Yes</Badge></td>
							<td class="actions"><Button variant="ghost">Enter</Button><Button variant="danger">Delete</Button></td>
						</tr>
						<tr>
							<td>Quiet moon</td>
							<td class="muted">a3f09c1e55d2b7c4</td>
							<td><Badge>No</Badge></td>
							<td class="actions"><Button variant="ghost">Enter</Button><Button variant="danger">Delete</Button></td>
						</tr>
					</tbody>
				</table>
			</Table>
			<Pager page={2} pages={5} />
		</section>

		<section>
			<h2>Panel and Stat</h2>
			<p class="muted">Floats in a corner of the engine canvas. The hatched box stands in for the canvas.</p>
			<div class="canvas">
				<Panel title="New planet">
					{#snippet aside()}<a href="/ds">planet</a>{/snippet}
					<dl>
						<Stat label="Frames" value="60" />
						<Stat label="Altitude" value="12.5 m" />
					</dl>
				</Panel>
				<Panel title="Bottom right" corner="bottom-right">
					<p>Any corner.</p>
				</Panel>
			</div>
		</section>

		<section>
			<h2>Topbar</h2>
			<p class="muted">The site bar is at the top of this page. The backoffice variant:</p>
			<div class="frame"><Topbar area="backoffice" /></div>
		</section>
	</div>
</Page>

<style>
	.sections {
		display: flex;
		flex-direction: column;
		gap: var(--sp-7);
	}
	section {
		display: flex;
		flex-direction: column;
		gap: var(--sp-4);
	}
	h2 {
		padding-bottom: var(--sp-2);
		border-bottom: var(--bw) solid var(--border);
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
	}
	.rules {
		display: flex;
		flex-direction: column;
		gap: var(--sp-2);
		max-width: var(--measure);
	}
	.rules li::before {
		content: '- ';
	}
	code {
		background: var(--bg-inset);
	}
	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--sp-3);
	}
	.form {
		max-width: var(--page-narrow);
	}
	.swatches {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(calc(3 * var(--sp-8)), 1fr));
		gap: var(--sp-3);
	}
	.swatch {
		display: flex;
		flex-direction: column;
		gap: var(--sp-2);
	}
	.swatch span {
		height: var(--sp-7);
		border: var(--bw) solid var(--border-strong);
	}
	.spaces {
		display: flex;
		flex-direction: column;
		gap: var(--sp-2);
	}
	.space {
		display: flex;
		align-items: center;
		gap: var(--sp-3);
	}
	.space span {
		height: var(--sp-3);
		background: var(--action);
	}
	.caps {
		font-weight: var(--fw-bold);
		letter-spacing: var(--ls-caps);
		text-transform: uppercase;
	}
	.strong {
		font-weight: var(--fw-bold);
	}
	.muted {
		color: var(--text-muted);
	}
	.canvas {
		position: relative;
		height: calc(5 * var(--sp-8));
		border: var(--bw) solid var(--border-strong);
		background: repeating-linear-gradient(45deg, var(--bg-inset) 0 var(--sp-3), var(--bg) var(--sp-3) var(--sp-5));
	}
	.frame {
		border: var(--bw) solid var(--border);
	}
</style>
