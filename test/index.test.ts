import { describe, it, expect } from 'vitest';
import { createSearchEngine } from '../dist/index.js';
import type { SearchIndex } from '../dist/index.js';

describe('Search Engine Test', () => {
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
});


