import { buildOptions } from './build-config.mjs';
import * as esbuild from 'esbuild';

export const ctx = await esbuild.context(buildOptions);
await ctx.watch();
