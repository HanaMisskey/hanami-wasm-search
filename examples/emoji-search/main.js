// main.js - メイン処理を行うスクリプト
import { EmojiIndex } from './emojiIndex.js';

// DOM要素
const searchInput = document.getElementById('search-input');
const resultsContainer = document.getElementById('results-container');
const statusMessage = document.getElementById('status-message');
const loadingIndicator = document.getElementById('loading-indicator');

// 絵文字インデックスのインスタンスを作成
const emojiIndex = new EmojiIndex();

// インデックスの初期化
async function initializeIndex() {
    showLoading('インデックスを初期化中...');
    
    try {
        await emojiIndex.initialize();
        showStatus(`${emojiIndex.getDocCount()}個の絵文字が読み込まれました`);
    } catch (error) {
        console.error('インデックス初期化エラー:', error);
        showStatus('絵文字データの読み込みに失敗しました');
    } finally {
        hideLoading();
    }
}

// 検索処理
async function searchEmojis(query) {
    if (!query.trim()) {
        clearResults();
        return;
    }
    
    showLoading('検索中...');
    
    try {
        const results = await emojiIndex.search(query);
        displayResults(results);
        
        const resultCount = results.length;
        showStatus(`${resultCount}件の絵文字が見つかりました`);
    } catch (error) {
        console.error('検索エラー:', error);
        showStatus('検索中にエラーが発生しました');
    } finally {
        hideLoading();
    }
}

// 検索結果を表示
function displayResults(results) {
    clearResults();
    
    if (results.length === 0) {
        showStatus('該当する絵文字が見つかりませんでした');
        return;
    }
    
    results.forEach(emoji => {
        const emojiCard = document.createElement('div');
        emojiCard.className = 'emoji-card';
        emojiCard.addEventListener('click', () => {
            navigator.clipboard.writeText(`:${emoji.name}:`);
            showStatus(`「:${emoji.name}:」をクリップボードにコピーしました`);
        });
        
        const emojiElement = document.createElement('div');
        emojiElement.className = 'emoji';
        
        // URLがある場合は画像として表示
        if (emoji.emoji && emoji.emoji.startsWith('http')) {
            const imgElement = document.createElement('img');
            imgElement.src = emoji.emoji;
            imgElement.alt = emoji.name;
            imgElement.className = 'emoji-img';
            emojiElement.appendChild(imgElement);
        } else {
            // URLがない場合はテキストとして表示（後方互換性）
            emojiElement.textContent = emoji.emoji || emoji.name;
        }
        
        const nameElement = document.createElement('div');
        nameElement.className = 'emoji-name';
        nameElement.textContent = emoji.name;
        
        emojiCard.appendChild(emojiElement);
        emojiCard.appendChild(nameElement);
        
        resultsContainer.appendChild(emojiCard);
    });
}

// 結果表示をクリア
function clearResults() {
    resultsContainer.innerHTML = '';
}

// ステータスメッセージを表示
function showStatus(message) {
    statusMessage.textContent = message;
}

// ローディング表示
function showLoading(message = '処理中...') {
    loadingIndicator.classList.remove('hidden');
    showStatus(message);
}

// ローディング非表示
function hideLoading() {
    loadingIndicator.classList.add('hidden');
}

// 検索処理のディバウンス
function debounce(func, wait) {
    let timeout;
    return function executedFunction(...args) {
        const later = () => {
            clearTimeout(timeout);
            func(...args);
        };
        clearTimeout(timeout);
        timeout = setTimeout(later, wait);
    };
}

// イベントリスナー
const debouncedSearch = debounce(searchEmojis, 300);

searchInput.addEventListener('input', (e) => {
    debouncedSearch(e.target.value);
});

// アプリケーション初期化
document.addEventListener('DOMContentLoaded', () => {
    initializeIndex();
});

// ショートカットキーの設定
document.addEventListener('keydown', (e) => {
    // Cmd+K または Ctrl+K でフォーカスを検索ボックスに移動
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        searchInput.focus();
    }
});
