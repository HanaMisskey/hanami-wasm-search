import { createSearchEngine } from '../dist/index.js';
import https from 'https';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

// __dirname を ESM で使用可能にする
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// JSONをキャッシュするファイルパス
const CACHE_FILE = path.join(__dirname, 'emojis_cache.json');

// パフォーマンス統計を計算するヘルパー関数
function calculateStats(times) {
  const sum = times.reduce((a, b) => a + b, 0);
  const mean = sum / times.length;
  const squareDiffs = times.map(time => Math.pow(time - mean, 2));
  const variance = squareDiffs.reduce((a, b) => a + b, 0) / times.length;
  const stdDev = Math.sqrt(variance);
  const min = Math.min(...times);
  const max = Math.max(...times);
  const sorted = [...times].sort((a, b) => a - b);
  const median = times.length % 2 === 0
    ? (sorted[times.length / 2 - 1] + sorted[times.length / 2]) / 2
    : sorted[Math.floor(times.length / 2)];

  return {
    mean,
    median,
    stdDev,
    min,
    max,
    total: sum,
    samples: times.length
  };
}

// 結果を整形して表示する関数
function formatStats(stats) {
  return `
  総時間: ${stats.total.toFixed(2)}ms
  平均時間: ${stats.mean.toFixed(2)}ms
  中央値: ${stats.median.toFixed(2)}ms
  標準偏差: ${stats.stdDev.toFixed(2)}ms
  最小: ${stats.min.toFixed(2)}ms
  最大: ${stats.max.toFixed(2)}ms
  サンプル数: ${stats.samples}
  `;
}

// キャッシュファイルからJSONを取得するか、ダウンロードする
async function getEmojisData() {
  // キャッシュがあればそれを使用
  if (fs.existsSync(CACHE_FILE)) {
    console.log('キャッシュからemojisデータを読み込み中...');
    return JSON.parse(fs.readFileSync(CACHE_FILE, 'utf8'));
  }

  console.log('misskey.flowers/api/emojis.jsonからデータをダウンロード中...');

  return new Promise((resolve, reject) => {
    const req = https.get('https://misskey.flowers/api/emojis', (res) => {
      if (res.statusCode !== 200) {
        reject(new Error(`ステータスコード: ${res.statusCode}`));
        return;
      }

      let data = '';
      res.on('data', (chunk) => {
        data += chunk;
      });

      res.on('end', () => {
        try {
          const jsonData = JSON.parse(data);
          // キャッシュに保存
          fs.writeFileSync(CACHE_FILE, data);
          resolve(jsonData);
        } catch (e) {
          reject(e);
        }
      });
    });

    req.on('error', (e) => {
      reject(e);
    });

    req.end();
  });
}

// time計測用の代替関数
const getNow = () => {
  return typeof performance !== 'undefined' ? performance.now() : Date.now();
};

// ベンチマークを実行する
async function runBenchmark() {
  console.log('=== Hanami WASM Search ベンチマーク ===');
  console.log('misskey.flowersのemojiデータを使用したベンチマーク\n');

  try {
    // データを取得
    console.log('データ取得を開始します...');
    const emojisData = await getEmojisData();

    // データの情報を表示
    console.log(`絵文字データの総数: ${emojisData.emojis.length}`);

    // ベンチマークの繰り返し回数
    const ITERATIONS = 20;

    // 検索クエリ用の単語リスト（すべての絵文字名とエイリアスを使用）
    const queryTerms = [];

    // すべての絵文字名を収集
    emojisData.emojis.forEach((emoji) => {
      // 絵文字名を追加
      queryTerms.push(emoji.name);

      // エイリアスも追加（存在する場合）
      if (emoji.aliases && emoji.aliases.length > 0) {
        emoji.aliases.forEach(alias => {
          queryTerms.push(alias);
        });
      }
    });

    console.log(`検索クエリ用語の総数: ${queryTerms.length}`);

    // メモリ使用量の問題を避けるため、検索回数を制限
    const MAX_SEARCH_COUNT = 100000;
    const SEARCH_COUNT = Math.min(queryTerms.length, MAX_SEARCH_COUNT);

    console.log(`実行する検索クエリ数: ${SEARCH_COUNT}`);

    // ランダムにクエリを選ぶためのヘルパー関数
    function getRandomQuery() {
      const randomIndex = Math.floor(Math.random() * queryTerms.length);
      return queryTerms[randomIndex];
    }

    // Hanami Search Engine を初期化
    console.log('Hanami Search Engine を初期化中...');
    const searchEngine = await createSearchEngine();
    console.log('Hanami Search Engine 初期化完了');

    // 統計情報を保存する配列
    const indexStats = [];
    const searchStats = [];
    const deleteStats = [];

    for (let i = 0; i < ITERATIONS; i++) {
      console.log(`\n--- ベンチマーク実行 ${i + 1}/${ITERATIONS} ---`);

      // インデックスの作成
      const indexStart = getNow();
      console.log('インデックスを作成中...');

      // データを追加
      console.log('データ処理中...');
      const processedData = {
        emojis: emojisData.emojis.map(emoji => ({
          name: emoji.name,
          aliases: emoji.aliases || []
        }))
      };

      console.log('インデックスにデータを追加中...');
      searchEngine.addDocuments(processedData);
      console.log('インデックスにデータを追加完了');
      const indexEnd = getNow();

      const indexTime = indexEnd - indexStart;
      console.log(`インデックス作成時間: ${indexTime.toFixed(2)}ms`);
      indexStats.push(indexTime);

      // 検索のベンチマーク
      console.log(`検索の実行中... (${SEARCH_COUNT}回)`);
      const searchTimes = [];

      // 進捗表示のためのカウンター
      const progressInterval = Math.max(1, Math.floor(SEARCH_COUNT / 20)); // 5%ごとに表示

      for (let j = 0; j < SEARCH_COUNT; j++) {
        // ランダムなクエリを取得
        const query = getRandomQuery();
        const searchStart = getNow();
        const results = await searchEngine.searchWithLimit(query, 10);
        const searchEnd = getNow();

        // 進捗表示
        if (j % progressInterval === 0 || j === SEARCH_COUNT - 1) {
          const progress = Math.round((j + 1) / SEARCH_COUNT * 100);
          console.log(`  検索進捗: ${progress}% (${j + 1}/${SEARCH_COUNT})`);
        }

        searchTimes.push(searchEnd - searchStart);
      }

      const searchStats_iteration = calculateStats(searchTimes);
      console.log(`検索の統計情報:${formatStats(searchStats_iteration)}`);
      searchStats.push(searchStats_iteration.mean);

      // 削除のベンチマーク
      console.log('削除の実行中...');
      const deleteStart = getNow();

      // すべての絵文字名を削除
      console.log('ドキュメントの削除を開始');
      emojisData.emojis.forEach((emoji, idx) => {
        try {
          searchEngine.removeDocument(emoji.name);
          if (idx % 1000 === 0) {
            console.log(`${idx}件のドキュメントを削除済み`);
          }
        } catch (e) {
          console.error('削除中にエラー:', e, emoji.name);
        }
      });

      const deleteEnd = getNow();
      const deleteTime = deleteEnd - deleteStart;
      console.log(`削除時間: ${deleteTime.toFixed(2)}ms`);
      deleteStats.push(deleteTime);

      // メモリ使用量をクリアするための対策
      if (global.gc) {
        global.gc();
      }
    }

    // 全体の統計を表示
    console.log('\n' + '='.repeat(50));
    console.log('       HANAMI WASM SEARCH ベンチマーク結果');
    console.log('='.repeat(50));

    const indexStatsCalc = calculateStats(indexStats);
    const searchStatsCalc = calculateStats(searchStats);
    const deleteStatsCalc = calculateStats(deleteStats);

    // 平均パフォーマンスを計算
    const avgIndexTime = indexStatsCalc.mean;
    const avgSearchTime = searchStatsCalc.mean;
    const avgDeleteTime = deleteStatsCalc.mean;

    // テーブルを作成して表示
    console.log('\n▼ 操作別パフォーマンス統計');
    console.log('-'.repeat(80));
    console.log('| 操作             | 平均 (ms) | 中央値 (ms) | 標準偏差 (ms) | 最小 (ms) | 最大 (ms) |');
    console.log('|' + '-'.repeat(78) + '|');
    console.log(`| インデックス作成 | ${avgIndexTime.toFixed(2).padStart(10)} | ${indexStatsCalc.median.toFixed(2).padStart(11)} | ${indexStatsCalc.stdDev.toFixed(2).padStart(13)} | ${indexStatsCalc.min.toFixed(2).padStart(9)} | ${indexStatsCalc.max.toFixed(2).padStart(9)} |`);
    console.log(`| 検索 (平均/回)   | ${avgSearchTime.toFixed(2).padStart(10)} | ${searchStatsCalc.median.toFixed(2).padStart(11)} | ${searchStatsCalc.stdDev.toFixed(2).padStart(13)} | ${searchStatsCalc.min.toFixed(2).padStart(9)} | ${searchStatsCalc.max.toFixed(2).padStart(9)} |`);
    console.log(`| 削除 (全${emojisData.emojis.length}件)  | ${avgDeleteTime.toFixed(2).padStart(10)} | ${deleteStatsCalc.median.toFixed(2).padStart(11)} | ${deleteStatsCalc.stdDev.toFixed(2).padStart(13)} | ${deleteStatsCalc.min.toFixed(2).padStart(9)} | ${deleteStatsCalc.max.toFixed(2).padStart(9)} |`);
    console.log('-'.repeat(80));

    // スループット計算
    const searchThroughput = (SEARCH_COUNT / (avgSearchTime / 1000)).toFixed(2);
    const deleteThroughput = (emojisData.emojis.length / (avgDeleteTime / 1000)).toFixed(2);

    console.log('\n▼ スループット (処理速度)');
    console.log('-'.repeat(50));
    console.log(`| 検索速度: ${searchThroughput.padStart(10)} 回/秒  |`);
    console.log(`| 削除速度: ${deleteThroughput.padStart(10)} 件/秒  |`);
    console.log('-'.repeat(50));

    // 詳細な統計情報
    console.log('\n▼ 詳細な統計情報');
    console.log('\nインデックス作成時間の統計:');
    console.log(formatStats(indexStatsCalc));

    console.log('検索時間の統計 (平均値に対する統計):');
    console.log(formatStats(searchStatsCalc));

    console.log('削除時間の統計:');
    console.log(formatStats(deleteStatsCalc));

    // レポートのまとめ
    console.log('\n' + '='.repeat(50));
    console.log(' ベンチマーク概要');
    console.log('-'.repeat(50));
    console.log(` - 実行回数: ${ITERATIONS}回`);
    console.log(` - データ総数: ${emojisData.emojis.length}件の絵文字データ`);
    console.log(` - 検索クエリの総数: ${queryTerms.length}種類`);
    console.log(` - 実行した検索クエリ数: ${SEARCH_COUNT}回`);
    console.log(` - 検索タイプ: 総当たり検索（全クエリからランダム選択）`);
    console.log(` - 実行環境: Node.js ${process.version}`);
    console.log('='.repeat(50));

  } catch (e) {
    console.error('ベンチマーク実行中にエラーが発生しました:', e);
    process.exitCode = 1;
  }
}

// Node.jsのバージョンとWASMサポート情報を表示
console.log('Node.js version:', process.version);
console.log('Initializing benchmark...');

// エラー処理を追加
process.on('uncaughtException', (err) => {
  console.error('未処理の例外:', err);
  process.exit(1);
});

// ベンチマーク実行
runBenchmark().catch(err => {
  console.error('ベンチマーク実行中のエラー:', err);
  process.exit(1);
});
