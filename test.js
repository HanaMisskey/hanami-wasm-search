#!/usr/bin/env node

const wasm = require('./pkg/hanami_wasm_search.js');

function runTests() {
  console.log('=== Search Engine Test Suite ===');
  
  // テスト関数
  function assertEqual(a, b, message) {
    if (JSON.stringify(a) !== JSON.stringify(b)) {
      console.error(`Failed: ${message}`);
      console.error(`  Expected: ${JSON.stringify(b)}`);
      console.error(`  Actual: ${JSON.stringify(a)}`);
      process.exitCode = 1;
    } else {
      console.log(`Passed: ${message}`);
    }
  }

  // インデックスのセットアップ
  function setupTestIndex() {
    const index = new wasm.Index();
    const testData = {
      emojis: [
        {
          name: 'smile',
          aliases: ['happy', 'joy']
        },
        {
          name: 'cry',
          aliases: ['sad', 'tear']
        },
        {
          name: '笑顔',
          aliases: ['えがお', 'スマイル']
        }
      ]
    };
    
    index.add_documents(JSON.stringify(testData));
    return index;
  }

  try {
    // テスト1: 空のインデックス検索
    console.log('\n--- Test: Empty Index ---');
    {
      const index = new wasm.Index();
      const queryJson = JSON.stringify(['test']);
      const results = index.searchWithLimit(queryJson, 10);
      console.log('Raw results from empty index:', results);
      assertEqual(results.length, 0, 'Empty index should return no results');
    }

    // テスト2: 完全一致検索
    console.log('\n--- Test: Exact Match ---');
    {
      const index = setupTestIndex();
      const queryJson = JSON.stringify(['smile']);
      const results = index.searchWithLimit(queryJson, 10);
      console.log('Raw results from exact match:', results);
      assertEqual(results.length, 1, 'Should have 1 result');
      assertEqual(results[0], 'smile', 'Result should be smile');
    }

    // テスト3: エイリアス検索
    console.log('\n--- Test: Alias Match ---');
    {
      const index = setupTestIndex();
      const queryJson = JSON.stringify(['happy']);
      const results = index.searchWithLimit(queryJson, 10);
      assertEqual(results.length, 1, 'Should have 1 result');
      assertEqual(results[0], 'smile', 'Result should be smile');
    }

    // テスト4: 部分一致検索
    console.log('\n--- Test: Partial Match ---');
    {
      const index = setupTestIndex();
      const queryJson = JSON.stringify(['smi']);
      const results = index.searchWithLimit(queryJson, 10);
      assertEqual(results.length, 1, 'Should have 1 result');
      assertEqual(results[0], 'smile', 'Result should be smile');
    }

    // テスト5: 日本語検索
    console.log('\n--- Test: Japanese Text ---');
    {
      const index = setupTestIndex();
      const queryJson = JSON.stringify(['笑顔']);
      const results = index.searchWithLimit(queryJson, 10);
      assertEqual(results.length, 1, 'Should have 1 result');
      assertEqual(results[0], '笑顔', 'Result should be 笑顔');
      
      // ひらがな検索
      const queryHiragana = JSON.stringify(['えがお']);
      const resultsHiragana = index.searchWithLimit(queryHiragana, 10);
      assertEqual(resultsHiragana.length, 1, 'Should have 1 result for hiragana');
      assertEqual(resultsHiragana[0], '笑顔', 'Result should be 笑顔 for hiragana search');
      
      // カタカナ検索
      const queryKatakana = JSON.stringify(['スマイル']);
      const resultsKatakana = index.searchWithLimit(queryKatakana, 10);
      assertEqual(resultsKatakana.length, 1, 'Should have 1 result for katakana');
      assertEqual(resultsKatakana[0], '笑顔', 'Result should be 笑顔 for katakana search');
    }

    // テスト6: ローマ字からひらがな変換
    console.log('\n--- Test: Romaji to Hiragana ---');
    {
      const index = setupTestIndex();
      const queryJson = JSON.stringify(['egao']);
      const results = index.searchWithLimit(queryJson, 10);
      assertEqual(results.length, 1, 'Should have 1 result for romaji conversion');
      assertEqual(results[0], '笑顔', 'Result should be 笑顔 for romaji conversion');
    }

    // テスト7: ドキュメント操作
    console.log('\n--- Test: Document Operations ---');
    {
      const index = new wasm.Index();
      
      // ドキュメント追加
      index.addDocument('にやけ', JSON.stringify(['歯茎', '惚気']));
      
      // 検索
      const queryJson = JSON.stringify(['歯茎']);
      const results = index.searchWithLimit(queryJson, 10);
      assertEqual(results.length, 1, 'Should find document after adding');
      assertEqual(results[0], 'にやけ', 'Result should be test1');
      
      // ドキュメント更新
      index.updateDocument('にやけ', JSON.stringify(['ニチャｱ…', 'ケアレスミス']));
      
      const oldResults = index.searchWithLimit(queryJson, 10);
      console.log('Search results with old alias after update:', oldResults);
      assertEqual(oldResults.length, 0, 'Should not find document with old alias');
      
      // 新しいエイリアスで見つかる
      const newQuery = JSON.stringify(['ニチャｱ…']);
      const newResults = index.searchWithLimit(newQuery, 10);
      assertEqual(newResults.length, 1, 'Should find document with new alias');
      assertEqual(newResults[0], 'にやけ', 'Result should be test1');
      
      // ドキュメント削除
      index.removeDocument('にやけ');
      
      // 削除後検索
      const afterDeleteResults = index.searchWithLimit(newQuery, 10);
      assertEqual(afterDeleteResults.length, 0, 'Should not find document after delete');
    }

    // テスト8: 1文字のnameとaliasの検索
    console.log('\n--- Test: Single Character Search ---');
    {
      const index = new wasm.Index();
      
      // 1文字のドキュメント名を追加
      index.addDocument('絵', JSON.stringify(['イラスト', '画']));
      
      // 1文字のaliasを持つドキュメントを追加
      index.addDocument('猫', JSON.stringify(['ねこ', '🐱', '猫科']));
      
      // 1文字のドキュメント名で検索
      const nameQuery = JSON.stringify(['絵']);
      const nameResults = index.searchWithLimit(nameQuery, 10);
      assertEqual(nameResults.length, 1, 'Should find document with single character name');
      assertEqual(nameResults[0], '絵', 'Result should be 絵');
      
      // 1文字のaliasで検索
      const aliasQuery = JSON.stringify(['🐱']);
      const aliasResults = index.searchWithLimit(aliasQuery, 10);
      assertEqual(aliasResults.length, 1, 'Should find document with single character alias');
      assertEqual(aliasResults[0], '猫', 'Result should be 猫');
      
      // 1文字の部分一致で検索
      const partialQuery = JSON.stringify(['画']);
      const partialResults = index.searchWithLimit(partialQuery, 10);
      assertEqual(partialResults.length, 1, 'Should find document with single character partial match');
      assertEqual(partialResults[0], '絵', 'Result should be 絵');
    }

    console.log('\n=== Test Summary ===');
    if (process.exitCode === 1) {
      console.log('Some tests failed');
    } else {
      console.log('All tests passed!');
    }
  } catch (e) {
    console.error('Test error:', e);
    process.exitCode = 1;
  }
}

// 実行
runTests();
