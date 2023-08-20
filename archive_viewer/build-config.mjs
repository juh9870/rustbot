import { htmlPlugin } from '@craftamap/esbuild-plugin-html';

/**
 * @type {import('esbuild').BuildOptions}
 */
export const buildOptions = {
  entryPoints: ['./src/index.tsx'],
  bundle: true,
  minify: true,
  sourcemap: true,
  logLevel: 'info',
  tsconfig: 'tsconfig.json',
  // target: ["chrome58", "firefox57", "safari11", "edge16"],
  // outfile: "dist/out.js",
  outdir: 'dist/',
  metafile: true,
  publicPath: '/',
  plugins: [
    htmlPlugin({
      files: [
        {
          entryPoints: ['src/index.tsx'],
          filename: 'archive.html',
          findRelatedCssFiles: true,
          inline: true,
          htmlTemplate: `
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <title>Archive viewer</title>
                <script>
                    window.jsonp_parse = (data) => window.jsonData = data;
                </script>
                <script type="text/javascript" src="messages.jsonp"></script>
            </head>
            <body>
<!--                <object data="messages.json" style="display: none;" onload="this.before(this.contentDocument.children[0]); this.remove();"></object>-->
            </body>
            </html>
          `,
        },
      ],
    }),
  ],
};
