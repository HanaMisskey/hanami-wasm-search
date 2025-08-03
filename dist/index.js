import { defu as i } from "defu";
function s(e) {
  return {
    setParams: (t, a) => e.set_params(t, a),
    addDocuments: (t) => e.add_documents(JSON.stringify(t)),
    search: (t, a) => e.search(JSON.stringify([t]), a),
    searchNoLimit: (t) => e.searchNoLimit(JSON.stringify([t])),
    searchWithLimit: (t, a) => e.searchWithLimit(JSON.stringify([t]), a),
    dump: () => e.dump(),
    removeDocument: (t) => e.removeDocument(t),
    addDocument: (t, a) => e.addDocument(t, JSON.stringify(a)),
    updateDocument: (t, a) => e.updateDocument(t, JSON.stringify(a)),
    clearIndex: () => e.clearIndex()
  };
}
async function o(e) {
  const t = i(e, {
    params: {
      k1: 1.2,
      b: 0.75
    },
    wasmInput: null,
    preCompiledIndex: null
  });
  let a;
  if (t.wasmInput != null)
    a = t.wasmInput;
  else {
    const { default: r } = await import("./chunks/hanami_wasm_search_bg.B1lz8jh2.js");
    a = await fetch(r);
  }
  const { default: m, Index: n } = await import("./chunks/hanami_wasm_search.sL9QEpCF.js");
  if (await m(a), t.preCompiledIndex != null) {
    const r = n.load(t.preCompiledIndex);
    return r.set_params(t.params.k1, t.params.b), s(r);
  } else {
    const r = n.newWithParams(t.params.k1, t.params.b);
    return s(r);
  }
}
export {
  o as createSearchEngine
};
