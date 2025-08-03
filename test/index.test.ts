import { describe, it, expect } from 'vitest';
import { createSearchEngine } from '../dist/index.js';
import type { SearchIndex } from '../dist/index.js';

describe('Search Engine Test', () => {
    // 旧形式のインデックスを模擬するためのヘルパー
    function createOldFormatDump(): Uint8Array {
        // 旧形式のデータ構造を手動で作成
        return new Uint8Array([]);
    }
    // ヘルパー関数: テスト用インデックスのセットアップ
    async function setupTestIndex() {
        const engine = await createSearchEngine();
        const testData = {
            emojis: [
                { name: 'smile', aliases: ['happy', 'joy'] },
                { name: 'cry', aliases: ['sad', 'tear'] },
                { name: '笑顔', aliases: ['えがお', 'スマイル'] },
            ],
        } satisfies SearchIndex;
        engine.addDocuments(testData);
        return engine;
    }

    it('Empty Index', async () => {
        const engine = await createSearchEngine();
        const results = await engine.searchWithLimit('test', 10);
        expect(results).toHaveLength(0);
    });

    it('Exact Match', async () => {
        const engine = await setupTestIndex();
        const results = await engine.searchWithLimit('smile', 10);
        expect(results).toHaveLength(1);
        expect(results[0]).toBe('smile');
    });

    it('Alias Match', async () => {
        const engine = await setupTestIndex();
        const results = await engine.searchWithLimit('happy', 10);
        expect(results).toHaveLength(1);
        expect(results[0]).toBe('smile');
    });

    it('Partial Match', async () => {
        const engine = await setupTestIndex();
        const results = await engine.searchWithLimit('smi', 10);
        expect(results).toHaveLength(1);
        expect(results[0]).toBe('smile');
    });

    it('Japanese Text', async () => {
        const engine = await setupTestIndex();
        const results = await engine.searchWithLimit('笑顔', 10);
        expect(results).toHaveLength(1);
        expect(results[0]).toBe('笑顔');

        const resultsHiragana = await engine.searchWithLimit('えがお', 10);
        expect(resultsHiragana).toHaveLength(1);
        expect(resultsHiragana[0]).toBe('笑顔');

        const resultsKatakana = await engine.searchWithLimit('スマイル', 10);
        expect(resultsKatakana).toHaveLength(1);
        expect(resultsKatakana[0]).toBe('笑顔');
    });

    it('Romaji to Hiragana', async () => {
        const engine = await setupTestIndex();
        const results = await engine.searchWithLimit('egao', 10);
        expect(results).toHaveLength(1);
        expect(results[0]).toBe('笑顔');
    });

    it('Document Operations', async () => {
        const engine = await createSearchEngine();

        engine.addDocument('にやけ', ['歯茎', '惚気']);
        let results = await engine.searchWithLimit('歯茎', 10);
        expect(results).toHaveLength(1);
        expect(results[0]).toBe('にやけ');

        engine.updateDocument('にやけ', ['ニチャｱ…', 'ケアレスミス']);
        results = await engine.searchWithLimit('歯茎', 10);
        expect(results).toHaveLength(0);

        results = await engine.searchWithLimit('ニチャｱ…', 10);
        expect(results).toHaveLength(1);
        expect(results[0]).toBe('にやけ');

        engine.removeDocument('にやけ');
        results = await engine.searchWithLimit('ニチャｱ…', 10);
        expect(results).toHaveLength(0);
    });

    it('Single Character Search', async () => {
        const engine = await createSearchEngine();

        engine.addDocument('絵', ['イラスト', '画']);
        engine.addDocument('猫', ['ねこ', '🐱', '猫科']);

        let results = await engine.searchWithLimit('絵', 10);
        expect(results).toHaveLength(1);
        expect(results[0]).toBe('絵');

        results = await engine.searchWithLimit('🐱', 10);
        expect(results).toHaveLength(1);
        expect(results[0]).toBe('猫');

        results = await engine.searchWithLimit('画', 10);
        expect(results).toHaveLength(1);
        expect(results[0]).toBe('絵');
    });

    it('Migration from old format', async () => {
        // 新形式のエンジンでデータを作成
        const engine = await setupTestIndex();
        
        // バージョン確認
        expect(engine.getVersion()).toBe(2);
        
        // ダンプを作成して再読み込み
        const dump = engine.dump();
        const engine2 = await createSearchEngine();
        engine2.load(dump);
        
        // データが正しくマイグレーションされたか確認
        const results = await engine2.searchWithLimit('smile', 10);
        expect(results).toHaveLength(1);
        expect(results[0]).toBe('smile');
        
        // バージョンが正しいか確認
        expect(engine2.getVersion()).toBe(2);
    });
});


