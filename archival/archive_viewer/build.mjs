import { buildOptions } from './build-config.mjs';
import { build } from 'esbuild';

await build(buildOptions);
