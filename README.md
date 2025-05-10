# hanami-wasm-search

軽量な WebAssembly ベースの日本語検索エンジン。特に絵文字検索に最適化されています。

[![npm version](https://badge.fury.io/js/hanami_wasm_search.svg)](https://www.npmjs.com/package/hanami_wasm_search)
[![Bundle Size](https://img.shields.io/bundlephobia/minzip/hanami_wasm_search)](https://bundlephobia.com/package/hanami_wasm_search)

## 特徴

- WebAssembly による高速な検索処理
- 日本語テキストの2-gramインデックス化
- ローマ字から日本語への変換サポート
- BM25アルゴリズムによる検索結果のランキング
- ブラウザ環境で動作（Node.js でも利用可能）
- 軽量で高速な実装

## インストール

```bash
npm install hanami_wasm_search
```

## 使用方法

### ブラウザ環境での使用

```js
import { Index } from 'hanami_wasm_search';

// 以下、初期化コードが続きます
```

### Node.js 環境での使用

```js
import { Index } from 'hanami_wasm_search';

// Node.js で使用する場合は特別な設定は不要です
// WebAssembly のロードは自動的に処理されます

async function main() {
  try {
    // インデックスの作成
    const index = new Index();
    
    // ドキュメントを追加
    const data = {
      emojis: [
        { name: "smile", aliases: ["笑顔", "スマイル", "にこにこ"] },
        { name: "heart", aliases: ["ハート", "愛", "こころ"] }
      ]
    };
    
    index.add_documents(JSON.stringify(data));
    
    // 検索を実行
    const results = index.search(JSON.stringify(["にこ"]), 10);
    console.log("検索結果:", results);
    
  } catch (e) {
    console.error("エラー:", e);
  }
}

main();
```

### インデックスの作成

```js
// インデックスをパラメータ付きで初期化（オプション）
const index = new Index(1.2, 0.75); // BM25 パラメータ: k1=1.2, b=0.75

// ドキュメントを追加
const emojisData = {
  emojis: [
    { name: "smile", aliases: ["笑顔", "スマイル", "にこにこ"] },
    { name: "heart", aliases: ["ハート", "愛", "こころ"] }
  ]
};
index.add_documents(JSON.stringify(emojisData));
```

### 検索の実行

```js
// 検索クエリを実行（結果の数を制限）
const results = index.search(JSON.stringify(["にこ"]), 10);
console.log(results); // ["smile", ...]

// 制限なしで検索
const allResults = index.searchNoLimit(JSON.stringify(["にこ"]));
console.log(allResults);

// 明示的に制限を指定して検索
const limitedResults = index.searchWithLimit(JSON.stringify(["にこ"]), 5);
console.log(limitedResults);
```

### インデックスの保存と読み込み

```js
// インデックスをバイナリ形式で保存
const serialized = index.dump();
localStorage.setItem('searchIndex', serialized);

// インデックスを読み込み
const savedIndex = localStorage.getItem('searchIndex');
if (savedIndex) {
  const index = Index.load(savedIndex);
}
```

## API リファレンス

### `new Index([k1, b])`

新しい検索インデックスを作成します。

- `k1` (省略可能): BM25 パラメータ (デフォルト: 1.2)
- `b` (省略可能): BM25 パラメータ (デフォルト: 0.75)

### `index.add_documents(jsonStr)`

JSON 文字列としてドキュメントを追加します。

- `jsonStr`: `{ "emojis": [{ "name": string, "aliases": string[] }] }` 形式の JSON 文字列

### `index.search(queryJsonStr, [limit])`

検索クエリを実行します。

- `queryJsonStr`: 検索キーワードの文字列配列の JSON 文字列
- `limit` (省略可能): 返す結果の最大数 (デフォルト: 20)

### `index.searchNoLimit(queryJsonStr)`

検索クエリを実行し、結果数の制限なしで返します。

- `queryJsonStr`: 検索キーワードの文字列配列の JSON 文字列

### `index.searchWithLimit(queryJsonStr, limit)`

検索クエリを実行し、結果数を制限します。

- `queryJsonStr`: 検索キーワードの文字列配列の JSON 文字列
- `limit`: 返す結果の最大数

### `index.dump()`

インデックスをバイナリ形式にシリアライズします。

### `Index.load(bytes)`

シリアライズされたインデックスを読み込みます。

- `bytes`: `dump()` メソッドで生成されたバイナリデータ

### `index.removeDocument(docId)`

インデックスから特定のドキュメントを削除します。

- `docId`: 削除するドキュメントの ID (name)

### `index.addDocument(name, aliasesJson)`

単一のドキュメントをインデックスに追加します。

- `name`: ドキュメント ID
- `aliasesJson`: 別名配列の JSON 文字列

### `index.updateDocument(docId, aliasesJson)`

既存のドキュメントを更新します。

- `docId`: 更新するドキュメントの ID
- `aliasesJson`: 新しい別名配列の JSON 文字列

### `index.clearIndex()`

インデックスを完全にクリアします。

## ブラウザでの使用例

```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <title>WASM Search Demo</title>
  <script type="module">
    import { Index } from './hanami_wasm_search.js';
    
    // WebAssembly モジュールの初期化完了後に実行
    async function init() {
      try {
        // インデックスの作成
        const index = new Index();
        
        // サンプルデータの追加
        const data = {
          emojis: [
            { name: "smile", aliases: ["笑顔", "スマイル", "にこにこ"] },
            { name: "heart", aliases: ["ハート", "愛", "こころ"] }
          ]
        };
        index.add_documents(JSON.stringify(data));
        
        // 検索の実行
        const searchInput = document.getElementById('searchInput');
        const resultsDiv = document.getElementById('results');
        
        document.getElementById('searchButton').addEventListener('click', () => {
          const query = searchInput.value;
          const results = index.search(JSON.stringify([query]), 10);
          
          resultsDiv.innerHTML = '';
          results.forEach(result => {
            const div = document.createElement('div');
            div.textContent = result;
            resultsDiv.appendChild(div);
          });
        });
        
        console.log('検索エンジンの初期化完了');
      } catch (e) {
        console.error('初期化エラー:', e);
      }
    }
    
    init();
  </script>
</head>
<body>
  <h1>WASM Search Demo</h1>
  <div>
    <input id="searchInput" type="text" placeholder="検索語を入力...">
    <button id="searchButton">検索</button>
  </div>
  <div id="results"></div>
</body>
</html>
```

## パッケージ最適化について

このパッケージは npm で公開するために最適化されています:

- 不要なデバッグログを削除
- 使用されていない依存関係を排除
- WebAssembly バイナリサイズの最適化
- 必要最小限のファイルのみを含む

## ライセンス

MIT

### シリアライズとデシリアライズ

```js
// インデックスの保存
const serialized = index.dump();

// インデックスの読み込み
const loadedIndex = Index.load(serialized);
```

## ビルド方法

```bash
# 開発用ビルド
wasm-pack build --target bundler

# 本番用ビルド
./build.sh
```

## デモ
// todo

ブラウザで <http://localhost:8000> を開き、検索語を入力してください。

## ライセンス

MIT