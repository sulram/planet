// Headless render to PNG with a fixed clock and seed. Arguments pass through:
//   bun run shot --out out/ground.png --seed 00000000deadbeef
//   bun run shot --out out/orbit.png --altitude 30000 --pitch -60 --boom 50
import { $ } from 'bun';
import { ROOT } from './lib';

$.cwd(ROOT);
const args = process.argv.slice(2);
if (!args.includes('--out')) args.push('--out', 'out/shot.png');
await $`cargo run --release -q -p shell-desktop -- shot ${args}`;
