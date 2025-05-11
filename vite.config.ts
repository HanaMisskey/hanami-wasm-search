import { resolve } from 'path';
import { defineConfig } from 'vite';
import { execa } from 'execa';
import dts from 'vite-plugin-dts';

const srcDir = resolve(import.meta.dirname, 'src');

export default defineConfig({
    plugins: [
        dts({
            exclude: ['test/**/*', 'src/wasm/**/*'],
        }),
        {
            name: 'plugin-build-before',
            buildStart: async () => {
                await execa('pnpm', ['build:wasm'], {
                    stdout: 'inherit',
                    stderr: 'inherit',
                });
            },
        },
    ],
    resolve: {
        alias: {
            '@': srcDir,
        },
    },
    build: {
        lib: {
            entry: resolve(srcDir, 'index.ts'),
            name: 'index',
            fileName: 'index',
            formats: ['es', 'cjs'],
        },
        rollupOptions: {
            external: ['defu'],
            output: {
                chunkFileNames: 'chunks/[name].[hash].js',
                assetFileNames: 'assets/[name][extname]',
            },
        },
    },
});
