// worker.js - WebWorkerで重い処理を行うスクリプト
// 純粋なJavaScriptでの実装

// グローバル変数
let emojiData = null;
let searchIndex = {};

// バイグラムインデックスの作成
function createBigramIndex(text) {
  if (!text || typeof text !== 'string') return [];
  
  const chars = [...text]; // 文字の配列に変換
  const bigrams = [];
  
  // 1文字だけの場合は、その文字だけを返す
  if (chars.length <= 1) {
    return chars.length === 0 ? [] : [text];
  }
  
  // バイグラム（2文字ごとのトークン）を生成
  for (let i = 0; i < chars.length - 1; i++) {
    bigrams.push(chars[i] + chars[i + 1]);
  }
  
  // 単語全体も追加
  bigrams.push(text);
  
  // 個別の文字も追加して部分一致を可能にする
  chars.forEach(char => {
    if (!bigrams.includes(char)) {
      bigrams.push(char);
    }
  });
  
  return bigrams;
}

// インデックスにドキュメントを追加
function addDocToIndex(docId, text, field = 'name') {
  // テキストを正規化（小文字に変換、トリミング）
  const normalizedText = text.toLowerCase().trim();
  
  // バイグラムを生成
  const tokens = createBigramIndex(normalizedText);
  
  // 各文字も個別にインデックスに追加
  const chars = [...normalizedText];
  
  // インデックスに追加
  tokens.forEach(token => {
    if (!searchIndex[token]) {
      searchIndex[token] = new Set();
    }
    searchIndex[token].add(docId);
  });
  
  // 各文字も個別にインデックスに追加（単一文字の検索用）
  chars.forEach(char => {
    if (!searchIndex[char]) {
      searchIndex[char] = new Set();
    }
    searchIndex[char].add(docId);
  });
}

// 絵文字データを取得
async function fetchEmojiData() {
  try {
    const response = await fetch('./emoji-data.json');
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return await response.json();
  } catch (error) {
    console.error('絵文字データの取得に失敗しました:', error);
    throw error;
  }
}

// インデックスを構築
async function buildIndex(emojis) {
  searchIndex = {};
  
  // 各絵文字をインデックスに追加
  for (const emoji of emojis) {
    // ドキュメントIDとして絵文字の名前を使用（新しいスキーマに対応）
    const docId = emoji.name;
    
    // 名前をインデックスに追加
    addDocToIndex(docId, emoji.name, 'name');
    
    // エイリアス（別名）があればインデックスに追加
    if (emoji.aliases && Array.isArray(emoji.aliases)) {
      for (const alias of emoji.aliases) {
        addDocToIndex(docId, alias, 'alias');
      }
    }
    
    // URLをインデックスに追加（オプション）
    if (emoji.url) {
      addDocToIndex(docId, emoji.url, 'url');
    }
    
    // カテゴリ情報があればインデックスに追加
    if (emoji.category) {
      addDocToIndex(docId, emoji.category, 'category');
    }
  }
  
  // Set をより効率的な配列に変換
  Object.keys(searchIndex).forEach(key => {
    searchIndex[key] = Array.from(searchIndex[key]);
  });
  
  return {
    docCount: emojis.length
  };
}

// インデックスの初期化
async function initialize() {
  try {
    // 絵文字データの取得
    emojiData = await fetchEmojiData();
    
    if (!emojiData || !emojiData.emojis || !Array.isArray(emojiData.emojis)) {
      throw new Error('無効な絵文字データ形式です');
    }
    
    // インデックスの構築
    const result = await buildIndex(emojiData.emojis);
    
    return {
      docCount: result.docCount,
      success: true
    };
  } catch (error) {
    console.error('インデックスの初期化エラー:', error);
    throw error;
  }
}

// 検索の実行
function search(query) {
  if (!emojiData || !emojiData.emojis) {
    throw new Error('インデックスがまだ初期化されていません');
  }
  
  if (!query || query.trim() === '') {
    return [];
  }
  
  try {
    // クエリを正規化
    const normalizedQuery = query.toLowerCase().trim();
    
    // クエリをバイグラムに分解
    const queryTokens = createBigramIndex(normalizedQuery);
    
    // 各トークンに一致するドキュメントIDを収集
    const matchingSets = queryTokens.map(token => {
      return searchIndex[token] || [];
    });
    
    // 一致するドキュメントがなければ空配列を返す
    if (matchingSets.length === 0 || matchingSets.every(set => set.length === 0)) {
      return [];
    }
    
    // 1文字検索の場合、すべての絵文字の中からその文字を含むものをフィルタリング
    if (normalizedQuery.length === 1) {
      const char = normalizedQuery;
      const allMatches = new Set();
      
      // インデックスを使用して効率的に検索
      Object.keys(searchIndex).forEach(token => {
        if (token.includes(char)) {
          searchIndex[token].forEach(docId => allMatches.add(docId));
        }
      });
      
      // 一致するものが見つからない場合は、インデックスがない可能性があるので全検索
      if (allMatches.size === 0) {
        emojiData.emojis.forEach(emoji => {
          if (emoji.name.toLowerCase().includes(char) || 
              (emoji.aliases && emoji.aliases.some(alias => alias.toLowerCase().includes(char)))) {
            allMatches.add(emoji.name);
          }
        });
      }
      
      if (allMatches.size > 0) {
        matchingSets.push([...allMatches]);
      }
    }
    
    // スコア計算のためのマップを作成
    const scoreMap = new Map();
    
    // 各トークンにマッチするドキュメントのスコアを加算
    matchingSets.forEach((docIds, i) => {
      const tokenWeight = i === matchingSets.length - 1 ? 2 : 1; // 完全一致は重みを高くする
      
      docIds.forEach(docId => {
        const currentScore = scoreMap.get(docId) || 0;
        scoreMap.set(docId, currentScore + tokenWeight);
      });
    });
    
    // スコアでソートした結果を生成
    const results = Array.from(scoreMap.entries())
      .sort((a, b) => b[1] - a[1]) // スコアの降順にソート
      .slice(0, 20) // 最大20件に制限
      .map(([docId, score]) => {
        const emojiObj = emojiData.emojis.find(e => e.name === docId);
        if (emojiObj) {
          return {
            emoji: emojiObj.url || "", // 新しいスキーマではURLを使用
            name: emojiObj.name,
            category: emojiObj.category || '',
            score: score
          };
        }
        return null;
      })
      .filter(Boolean);
    
    return results;
  } catch (error) {
    console.error('検索エラー:', error);
    throw error;
  }
}

// メッセージハンドラ
self.onmessage = async (event) => {
    const { id, type, data } = event.data;
    
    try {
        let result;
        
        switch (type) {
            case 'initialize':
                result = await initialize();
                break;
            case 'search':
                result = search(data.query);
                break;
            default:
                throw new Error(`不明なコマンドタイプ: ${type}`);
        }
        
        self.postMessage({ id, result });
    } catch (error) {
        console.error(`Worker error (${type}):`, error);
        self.postMessage({
            id,
            error: error.message || 'Unknown error'
        });
    }
};
