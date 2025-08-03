import { InitInput } from './wasm/hanami_wasm_search.js';
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
export declare function createSearchEngine(opts?: SearchEngineConfig): Promise<SearchEngineInstance>;
export {};
