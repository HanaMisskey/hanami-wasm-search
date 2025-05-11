// EmojiIndex.js - WebWorkerを使って絵文字インデックスを管理するクラス
export class EmojiIndex {
    constructor() {
        // WebWorkerの作成
        this.worker = new Worker('./worker.js', { type: 'module' });
        
        // プロミスを保持するマップ
        this.promises = new Map();
        this.nextId = 0;
        
        // Workerからのメッセージハンドラ
        this.worker.onmessage = (event) => {
            const { id, result, error } = event.data;
            
            if (this.promises.has(id)) {
                const { resolve, reject } = this.promises.get(id);
                
                if (error) {
                    reject(new Error(error));
                } else {
                    resolve(result);
                }
                
                this.promises.delete(id);
            }
        };
        
        // エラーハンドラ
        this.worker.onerror = (error) => {
            console.error('Worker error:', error);
        };
        
        this.docCount = 0;
    }
    
    // WebWorkerにメッセージを送信し、結果を待つヘルパーメソッド
    async _sendMessageToWorker(type, data = {}) {
        const id = this.nextId++;
        
        const promise = new Promise((resolve, reject) => {
            this.promises.set(id, { resolve, reject });
        });
        
        this.worker.postMessage({
            id,
            type,
            data
        });
        
        return promise;
    }
    
    // インデックスを初期化
    async initialize() {
        const result = await this._sendMessageToWorker('initialize');
        this.docCount = result.docCount;
        return result;
    }
    
    // 検索を実行
    async search(query) {
        if (!query || query.trim() === '') {
            return [];
        }
        return this._sendMessageToWorker('search', { query });
    }
    
    // インデックス内のドキュメント数を取得
    getDocCount() {
        return this.docCount;
    }
    
    // リソースの解放
    terminate() {
        if (this.worker) {
            this.worker.terminate();
            this.worker = null;
        }
    }
}
