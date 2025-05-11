//@ts-check

import { execa } from 'execa';
import which from 'which';
import { resolve } from 'node:path';
import { rm, copyFile } from 'node:fs/promises';

async function main() {
    if ((await which('wasm-pack', { nothrow: true })) == null) {
        console.error('wasm-pack not found, please install it first.');
        process.exit(1);
    }

    // Remove the dist directory
    await rm('src/wasm', { recursive: true, force: true });

    await execa('wasm-pack', ['build',
        '--target', 'web',
        '--out-dir', '../src/wasm',
        '--no-pack',
        '--release',
    ], {
        cwd: resolve(import.meta.dirname, '../crate'),
        stdout: 'inherit',
        stderr: 'inherit',
    });

    if ((await which('wasm-opt', { nothrow: true })) != null) {
        await execa('wasm-opt', ['-Oz', '--output', 'hanami_wasm_search_bg.wasm.opt', 'hanami_wasm_search_bg.wasm'], {
            cwd: resolve(import.meta.dirname, '../src/wasm'),
            stdout: 'inherit',
            stderr: 'inherit',
        });
        await rm('src/wasm/hanami_wasm_search_bg.wasm', { force: true });
        await copyFile('src/wasm/hanami_wasm_search_bg.wasm.opt', 'src/wasm/hanami_wasm_search_bg.wasm');
        await rm('src/wasm/hanami_wasm_search_bg.wasm.opt', { force: true });
    } else {
        console.warn('wasm-opt not found, skipping optimization');
    }

    // delete the .gitignore file
    await rm('src/wasm/.gitignore', { force: true });
}

main().catch((err) => {
    console.error(err);
    process.exit(1);
});
