# @hanamisskey/browser-search

軽量な WebAssembly ベースの日本語検索エンジン。特に絵文字検索に最適化されています。

## 特徴

- WebAssembly による高速な検索処理
- 日本語テキストの2-gramインデックス化
- ローマ字から日本語への変換サポート
- BM25アルゴリズムによる検索結果のランキング
- ブラウザ環境で動作（Node.js でも利用可能）
- 軽量で高速な実装

## 使用方法

### ブラウザ環境での使用

```js
import { createSearchEngine } from '@hanamisskey/browser-search';

// 以下、初期化コードが続きます
```

### Node.js 環境での使用

```js
import { createSearchEngine } from '@hanamisskey/browser-search';

// Node.js で使用する場合は特別な設定は不要です
// WebAssembly のロードは自動的に処理されます

async function main() {
  try {
    // 検索エンジンの作成
    const engine = await createSearchEngine();
    
    // ドキュメントを追加
    const data = {
      emojis: [
        { name: "smile", aliases: ["笑顔", "スマイル", "にこにこ"] },
        { name: "heart", aliases: ["ハート", "愛", "こころ"] }
      ]
    };
    
    engine.addDocuments(data);
    
    // 検索を実行
    const results = await engine.search("にこ", 10);
    console.log("検索結果:", results);
    
  } catch (e) {
    console.error("エラー:", e);
  }
}

main();
```

#### WebAssembly モジュールの初期化

WebAssembly モジュールはインラインされたソースから自動的に初期化されますが、必要に応じて手動で初期化することも可能です。

createSearchEngine のオプションに `wasmInput` を指定することで、別途読み込んだモジュールを使用できます。

初期化（Instantiate）が完了しているものではなく、WASMモジュール本体を指定する必要があります。

```ts
import { createSearchEngine } from '@hanamisskey/browser-search';
import wasmUrl from '@hanamisskey/browser-search/engine.wasm?url';

async function init() {
  const engine = await createSearchEngine({
    wasmInput: await fetch(wasmUrl),
  });
  
  // 検索エンジンの初期化が完了したら、ドキュメントを追加したり検索を実行できます
}
```

### インデックスの作成

```js
// 検索エンジンをパラメータ付きで初期化（オプション）
const engine = await createSearchEngine({
  params: { k1: 1.2, b: 0.75 }
});

// ドキュメントを追加
const emojisData = {
  emojis: [
    { name: "smile", aliases: ["笑顔", "スマイル", "にこにこ"] },
    { name: "heart", aliases: ["ハート", "愛", "こころ"] }
  ]
};
engine.addDocuments(emojisData);
```

### 検索の実行

```js
// 検索クエリを実行（結果の数を制限）
const results = await engine.search("にこ", 10);
console.log(results); // ["smile", ...]

// 制限なしで検索
const allResults = await engine.searchNoLimit("にこ");
console.log(allResults);

// 明示的に制限を指定して検索
const limitedResults = await engine.searchWithLimit("にこ", 5);
console.log(limitedResults);
```

### インデックスの保存と読み込み

```js
// インデックスをバイナリ形式で保存
const serialized = engine.dump();
localStorage.setItem('searchIndex', serialized);

// インデックスを読み込み
const savedIndex = localStorage.getItem('searchIndex');
if (savedIndex) {
  const engine = await createSearchEngine({ preCompiledIndex: savedIndex });
}
```

## API リファレンス

### `createSearchEngine([config])`

新しい検索エンジンインスタンスを作成します。

- `config` (省略可能): 検索エンジンの設定オブジェクト
  - `params`: BM25 パラメータ (デフォルト: `{ k1: 1.2, b: 0.75 }`)
  - `wasmModule`: WebAssembly モジュール (省略可能)
  - `preCompiledIndex`: 事前コンパイル済みインデックス (省略可能)

### `engine.setParams(k1, b)`

BM25 パラメータを設定します。

- `k1`: BM25 パラメータ
- `b`: BM25 パラメータ

### `engine.addDocuments(index)`

ドキュメントを追加します。

- `index`: `{ emojis: [{ name: string, aliases: string[] }] }` 形式のオブジェクト

### `engine.search(query, [limit])`

検索クエリを実行します。

- `query`: 検索キーワードの文字列
- `limit` (省略可能): 返す結果の最大数 (デフォルト: 20)

### `engine.searchNoLimit(query)`

検索クエリを実行し、結果数の制限なしで返します。

- `query`: 検索キーワードの文字列

### `engine.searchWithLimit(query, limit)`

検索クエリを実行し、結果数を制限します。

- `query`: 検索キーワードの文字列
- `limit`: 返す結果の最大数

### `engine.dump()`

インデックスをバイナリ形式にシリアライズします。

### `engine.removeDocument(name)`

インデックスから特定のドキュメントを削除します。

- `name`: 削除するドキュメントの ID

### `engine.addDocument(name, aliases)`

単一のドキュメントをインデックスに追加します。

- `name`: ドキュメント ID
- `aliases`: 別名の配列

### `engine.updateDocument(name, aliases)`

既存のドキュメントを更新します。

- `name`: 更新するドキュメントの ID
- `aliases`: 新しい別名の配列

### `engine.clearIndex()`

インデックスを完全にクリアします。

### シリアライズとデシリアライズ

```js
// インデックスの保存
const serialized = index.dump();

// インデックスの読み込み
const loadedIndex = Index.load(serialized);
```

## ビルド方法

`pnpm`・`wasm-pack`・`wasm-opt` が必要です。

```bash
# wasm-pack のインストール
cargo install wasm-pack
# wasm-opt のインストール
cargo install wasm-opt

# 依存関係のインストール
pnpm install
# ビルド
pnpm build
```

## ライセンス

MIT
