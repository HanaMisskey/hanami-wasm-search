import { defu } from 'defu';
import { Index } from '@/wasm/hanami_wasm_search.js';
import type { InitInput } from '@/wasm/hanami_wasm_search.js';

type DeepPartial<T> = {
    [K in keyof T]?: T[K] extends Record<PropertyKey, unknown> ? DeepPartial<T[K]> : T[K] extends null ? undefined : T[K] | undefined;
};

type _SearchEngineConfig = {
    wasmInput: InitInput | null;
    preCompiledIndex: Uint8Array | null;
};

export type SearchEngineConfig = DeepPartial<_SearchEngineConfig>;

export type SearchIndex = {
    emojis: {
        name: string;
        aliases: string[];
    }[];
};

export type SearchEngineInstance = {
    addDocuments: (idx: SearchIndex) => void;
    search: (query: string, limit?: number) => Promise<string[]>;
    searchNoLimit: (query: string) => Promise<string[]>;
    searchWithLimit: (query: string, limit: number) => Promise<string[]>;
    dump: () => Uint8Array;
    load: (data: Uint8Array) => void;
    removeDocument: (name: string) => boolean;
    addDocument: (name: string, aliases: string[]) => void;
    updateDocument: (name: string, aliases: string[]) => boolean;
    clearIndex: () => void;
    getVersion: () => number;
};

function packIndexInstance(index: Index): SearchEngineInstance {
    return {
        addDocuments: (idx: SearchIndex) => index.add_documents(JSON.stringify(idx)),
        search: (query: string, limit?: number) => index.search(JSON.stringify([query]), limit),
        searchNoLimit: (query: string) => index.searchNoLimit(JSON.stringify([query])),
        searchWithLimit: (query: string, limit: number) => index.searchWithLimit(JSON.stringify([query]), limit),
        dump: () => index.dump(),
        load: (data: Uint8Array) => {
            const newIndex = Index.load(data);
            Object.assign(index, newIndex);
        },
        removeDocument: (name: string) => index.removeDocument(name),
        addDocument: (name: string, aliases: string[]) => index.addDocument(name, JSON.stringify(aliases)),
        updateDocument: (name: string, aliases: string[]) => index.updateDocument(name, JSON.stringify(aliases)),
        clearIndex: () => index.clearIndex(),
        getVersion: () => index.getVersion(),
    };
}

export async function createSearchEngine(opts?: SearchEngineConfig): Promise<SearchEngineInstance> {
    const _opts = defu<_SearchEngineConfig, SearchEngineConfig[]>(opts, {
        wasmInput: null,
        preCompiledIndex: null,
    });

    let wasmInput: InitInput;

    if (_opts.wasmInput != null) {
        // WebAssemblyインスタンスが渡されたらそれを使用
        wasmInput = _opts.wasmInput;
    } else {
        const { default: wasmUrl } = await import('@/wasm/hanami_wasm_search_bg.wasm?url');
        wasmInput = await fetch(wasmUrl);
    }

    const { default: init, Index } = await import('@/wasm/hanami_wasm_search.js');
    await init(wasmInput);

    if (_opts.preCompiledIndex != null) {
        // プレコンパイルされたインデックスが渡されたらそれを使用して初期化
        const index = Index.load(_opts.preCompiledIndex);
        return packIndexInstance(index);
    } else {
        // プレコンパイルされたインデックスが渡されなかったら新規作成
        const index = new Index();
        return packIndexInstance(index);
    }
}
