import { defu } from 'defu';
import type { Index, InitInput } from '@/wasm/hanami_wasm_search.js';

type DeepPartial<T> = {
    [K in keyof T]?: T[K] extends Record<PropertyKey, unknown> ? DeepPartial<T[K]> : T[K] extends null ? undefined : T[K] | undefined;
};

type _SearchEngineConfig = {
    params: {
        k1: number;
        b: number;
    };
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
    setParams: (k1: number, b: number) => void;
    addDocuments: (idx: SearchIndex) => void;
    search: (query: string, limit?: number) => Promise<string[]>;
    searchNoLimit: (query: string) => Promise<string[]>;
    searchWithLimit: (query: string, limit: number) => Promise<string[]>;
    dump: () => Uint8Array;
    removeDocument: (name: string) => boolean;
    addDocument: (name: string, aliases: string[]) => void;
    updateDocument: (name: string, aliases: string[]) => boolean;
    clearIndex: () => void;
};

function packIndexInstance(index: Index): SearchEngineInstance {
    return {
        setParams: (k1: number, b: number) => index.set_params(k1, b),
        addDocuments: (idx: SearchIndex) => index.add_documents(JSON.stringify(idx)),
        search: (query: string, limit?: number) => index.search(JSON.stringify([query]), limit),
        searchNoLimit: (query: string) => index.searchNoLimit(JSON.stringify([query])),
        searchWithLimit: (query: string, limit: number) => index.searchWithLimit(JSON.stringify([query]), limit),
        dump: () => index.dump(),
        removeDocument: (name: string) => index.removeDocument(name),
        addDocument: (name: string, aliases: string[]) => index.addDocument(name, JSON.stringify(aliases)),
        updateDocument: (name: string, aliases: string[]) => index.updateDocument(name, JSON.stringify(aliases)),
        clearIndex: () => index.clearIndex(),
    };
}

export async function createSearchEngine(opts?: SearchEngineConfig): Promise<SearchEngineInstance> {
    const _opts = defu<_SearchEngineConfig, SearchEngineConfig[]>(opts, {
        params: {
            k1: 1.2,
            b: 0.75,
        },
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
        index.set_params(_opts.params.k1, _opts.params.b);
        return packIndexInstance(index);
    } else {
        // プレコンパイルされたインデックスが渡されなかったら新規作成
        const index = Index.newWithParams(_opts.params.k1, _opts.params.b);
        return packIndexInstance(index);
    }
}
