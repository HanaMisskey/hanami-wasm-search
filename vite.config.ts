import { resolve } from 'path';
import { copyFile, writeFile } from 'fs/promises';
import { defineConfig } from 'vite';
import { execa } from 'execa';
import dts from 'vite-plugin-dts';

const srcDir = resolve(import.meta.dirname, 'src');
const distDir = resolve(import.meta.dirname, 'dist');

export default defineConfig({
    plugins: [
        dts({
            exclude: ['test/**/*', 'src/wasm/**/*'],
        }),
        {
            name: 'plugin-hooks',
            options: async () => {
                await execa('pnpm', ['build:wasm'], {
                    stdout: 'inherit',
                    stderr: 'inherit',
                });
            },
            writeBundle: async () => {
                await copyFile(resolve(srcDir, 'wasm/hanami_wasm_search_bg.wasm'), resolve(distDir, 'engine.wasm'));
                await writeFile(resolve(distDir, 'engine.d.ts'), 'declare const binary: ArrayBuffer; export default binary;', 'utf-8');
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
