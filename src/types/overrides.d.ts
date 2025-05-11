declare module '@/wasm/hanami_wasm_search_bg.js' {
    export * from '@/wasm/hanami_wasm_search.js';
    export function __wbg_set_wasm(wasm: WebAssembly.Instance): typeof import('@/wasm/hanami_wasm_search_bg.wasm');
}