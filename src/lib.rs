use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use wana_kana::ConvertJapanese;

// 2-gram（バイグラム）トークン化関数
fn tokenize_2gram(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    
    // 日本語/英数字の文字単位でトークン化
    let chars: Vec<char> = text.chars().collect();
    
    // 文字数が1以下の場合は、そのまま返す
    if chars.len() <= 1 {
        if !chars.is_empty() {
            tokens.push(text.to_string());
        }
        return tokens;
    }
    
    // 2-gramトークンを生成
    for i in 0..chars.len() - 1 {
        let token: String = chars[i..=i+1].iter().collect();
        tokens.push(token);
    }
    
    // 元の単語自体も追加（完全一致検索のため）
    tokens.push(text.to_string());
    
    tokens
}

#[derive(Debug, Deserialize)]
struct Doc {
    name: String,
    aliases: Vec<String>,
}

// Root JSON structure
#[derive(Debug, Deserialize)]
struct EmojisData {
    emojis: Vec<Doc>,
}

// Changed from Vec<u32> to Vec<String> for doc IDs
type Postings = HashMap<String, Vec<String>>;

#[wasm_bindgen]
#[derive(Serialize, Deserialize)]
pub struct Index {
    postings: Postings,
    doc_len: HashMap<String, usize>, // Changed from u32 to String for doc IDs
    n_docs: usize,
    k1: f32,
    b: f32,
}

// JSONパースエラーをより詳細に出力するヘルパー関数
fn log_json_error(json: &str, error: &serde_json::Error) -> String {
    let error_msg = format!("JSON parse error at line {}, column {}: {}", 
                           error.line(), error.column(), error);
    
    // エラーが発生した周辺の文字列を抽出して表示
    let context_start = std::cmp::max(0, error.column() as i64 - 20) as usize;
    let context_end = std::cmp::min(json.len(), error.column() + 20);
    let context = if context_end > context_start {
        &json[context_start..context_end]
    } else {
        ""
    };
    
    // ログ出力を削除し、エラーメッセージのみを返す
    format!("{}\nContext: '{}'", error_msg, context)
}

#[wasm_bindgen]
impl Index {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Index {
        Self::with_params(1.2, 0.75)
    }

    #[wasm_bindgen(js_name = "newWithParams")]
    pub fn with_params(k1: f32, b: f32) -> Index {
        Index { postings: HashMap::new(), doc_len: HashMap::new(), n_docs: 0, k1, b }
    }

    pub fn set_params(&mut self, k1: f32, b: f32) { self.k1 = k1; self.b = b; }

    pub fn add_documents(&mut self, json: &str) -> Result<(), JsValue> {
        let data: EmojisData = match serde_json::from_str(json) {
            Ok(data) => data,
            Err(e) => {
                return Err(JsValue::from_str(&log_json_error(json, &e)));
            }
        };
        
        // 変数未使用警告を修正
        let _emoji_count = data.emojis.len();
        
        for doc in data.emojis {
            if self.doc_len.contains_key(&doc.name) { self.remove_doc(doc.name.clone()); }
            // 名前も含めたトークン数を記録
            self.doc_len.insert(doc.name.clone(), doc.aliases.len() + 1); // +1 for name
            self.n_docs += 1;
            
            let mut seen = HashSet::new();
            // 名前自体もインデックスに追加
            seen.insert(doc.name.clone());
            self.postings.entry(doc.name.clone()).or_default().push(doc.name.clone());
            
            // 名前の2-gramトークン化（部分一致用）
            let name_tokens = tokenize_2gram(&doc.name);
            for token in name_tokens {
                if !seen.insert(token.clone()) { continue; }
                self.postings.entry(token).or_default().push(doc.name.clone());
            }
            
            // エイリアス（別名）もインデックス化
            for token in doc.aliases {
                if !seen.insert(token.clone()) { continue; }
                self.postings.entry(token.clone()).or_default().push(doc.name.clone());
                
                // エイリアスも2-gramトークン化（部分一致用）
                let alias_tokens = tokenize_2gram(&token);
                for subtoken in alias_tokens {
                    if !seen.insert(subtoken.clone()) { continue; }
                    self.postings.entry(subtoken).or_default().push(doc.name.clone());
                }
            }
        }
        
        Ok(())
    }

    #[wasm_bindgen(js_name = "search")]
    pub fn search(&self, query_json: &str, limit: Option<usize>) -> Result<JsValue, JsValue> {
        let original: Vec<String> = serde_json::from_str(query_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        // デフォルトの検索結果制限を20に設定
        let result_limit = limit.unwrap_or(20);
        
        // クエリを2-gramに変換する
        let mut expanded_2gram: Vec<String> = Vec::new();
        for term in &original {
            // オリジナルの単語も検索対象に含める
            expanded_2gram.push(term.clone());
            
            // 日本語変換のサポート
            if term.chars().all(|c| c.is_ascii_alphabetic()) {
                let hira = term.to_lowercase().to_hiragana();
                if hira != *term {
                    expanded_2gram.push(hira.clone());
                    // 変換された日本語も2-gramトークン化
                    let hira_tokens = tokenize_2gram(&hira);
                    for token in hira_tokens {
                        if !expanded_2gram.contains(&token) {
                            expanded_2gram.push(token);
                        }
                    }
                }
            }
            
            // 2-gramトークン化
            let tokens = tokenize_2gram(term);
            for token in tokens {
                if !expanded_2gram.contains(&token) {
                    expanded_2gram.push(token);
                }
            }
        }
        
        let terms = &expanded_2gram;
        
        if self.n_docs == 0 {
            return Ok(serde_wasm_bindgen::to_value(&Vec::<String>::new()).unwrap());
        }
        let avg_len = self.doc_len.values().copied().sum::<usize>() as f32 / self.n_docs as f32;
        
        // スコア計算のために各ドキュメントごとにマッチの種類を記録
        let mut match_types: HashMap<String, (bool, bool, bool, bool)> = HashMap::new();
        
        // BM25スコア計算のベースとなる通常のスコア
        let mut scores: HashMap<String, f32> = HashMap::new();
        
        // 完全一致と部分一致を検出
        for term in terms {
            if let Some(list) = self.postings.get(term) {
                let df = list.len() as f32;
                let idf = ((self.n_docs as f32 - df + 0.5)/(df+0.5) + 1.0).ln();
                
                for doc_id in list {
                    // 基本的なBM25スコア計算
                    let tf = if original.contains(term) { 2.0 } else { 1.0 };
                    let len = *self.doc_len.get(doc_id).unwrap_or(&1) as f32;
                    let score = idf * (tf*(self.k1+1.0)) /
                        (tf + self.k1*(1.0 - self.b + self.b*len/avg_len));
                    *scores.entry(doc_id.clone()).or_insert(0.0) += score;
                    
                    // マッチ種類を記録 (name完全一致, alias完全一致, name部分一致, alias部分一致)
                    let entry = match_types.entry(doc_id.clone()).or_insert((false, false, false, false));
                    
                    // 完全一致の判定
                    if original.contains(term) {
                        if term == doc_id {
                            // nameと完全一致
                            entry.0 = true;
                        } else {
                            // aliasとの完全一致チェック
                            entry.1 = true;
                        }
                    } else {
                        // 部分一致の判定 (name)
                        if tokenize_2gram(doc_id).contains(term) {
                            entry.2 = true;
                        } else {
                            // aliasとの部分一致と判断
                            entry.3 = true;
                        }
                    }
                }
            }
        }
        // フィルタリングとスコアリング
        let mut results: Vec<(String, f32, (bool, bool, bool, bool))> = scores.into_iter()
            .map(|(doc_id, score)| {
                let match_type = match_types.get(&doc_id).cloned().unwrap_or((false, false, false, false));
                (doc_id, score, match_type)
            })
            .collect();
            
        // 優先順位付けでソート
        // 1. nameに対する完全一致
        // 2. aliasに対する完全一致
        // 3. nameに対する部分一致
        // 4. aliasに対する部分一致
        results.sort_by(|a, b| {
            let (_, _, a_match) = a;
            let (_, _, b_match) = b;
            
            // nameと完全一致
            match (a_match.0, b_match.0) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }
            
            // aliasと完全一致
            match (a_match.1, b_match.1) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }
            
            // nameと部分一致
            match (a_match.2, b_match.2) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }
            
            // aliasと部分一致
            match (a_match.3, b_match.3) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }
            
            // 同じ優先度の場合はBM25スコアで比較
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // 検索結果を制限して、IDのみを返す
        let ranked: Vec<String> = results.into_iter()
            .take(result_limit) // 検索結果を指定された数に制限する
            .map(|(id, _, _)| id)
            .collect();
        Ok(serde_wasm_bindgen::to_value(&ranked).unwrap())
    }

    #[wasm_bindgen(js_name = "searchNoLimit")]
    pub fn search_no_limit(&self, query_json: &str) -> Result<JsValue, JsValue> {
        // 元の関数をラップし、制限なし（None）で呼び出す
        self.search(query_json, None)
    }

    #[wasm_bindgen(js_name = "searchWithLimit")]
    pub fn search_with_limit(&self, query_json: &str, limit: usize) -> Result<JsValue, JsValue> {
        // 明示的に制限数を指定して検索
        self.search(query_json, Some(limit))
    }

    pub fn dump(&self) -> Result<js_sys::Uint8Array, JsValue> {
        Ok(js_sys::Uint8Array::from(
            &bincode::serialize(self).map_err(|e| JsValue::from_str(&e.to_string()))?[..],
        ))
    }
    pub fn load(bytes: js_sys::Uint8Array) -> Result<Index, JsValue> {
        bincode::deserialize(&bytes.to_vec()).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    fn remove_doc(&mut self, doc_id: String) {
        for list in self.postings.values_mut() { list.retain(|id| *id != doc_id); }
        self.doc_len.remove(&doc_id); self.n_docs -= 1;
    }

    #[wasm_bindgen(js_name = "removeDocument")]
    pub fn remove_document(&mut self, doc_id: &str) -> Result<bool, JsValue> {
        if self.doc_len.contains_key(doc_id) {
            self.remove_doc(doc_id.to_string());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    #[wasm_bindgen(js_name = "addDocument")]
    pub fn add_document(&mut self, name: &str, aliases_json: &str) -> Result<(), JsValue> {
        let aliases: Vec<String> = serde_json::from_str(aliases_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        // 既存のドキュメントなら削除
        if self.doc_len.contains_key(name) { 
            self.remove_doc(name.to_string()); 
        }
        
        // 名前も含めたトークン数を記録
        self.doc_len.insert(name.to_string(), aliases.len() + 1); // +1 for name
        self.n_docs += 1;
        
        let mut seen = HashSet::new();
        // 名前自体もインデックスに追加
        seen.insert(name.to_string());
        self.postings.entry(name.to_string()).or_default().push(name.to_string());
        
        // 名前の2-gramトークン化（部分一致用）
        let name_tokens = tokenize_2gram(name);
        for token in name_tokens {
            if !seen.insert(token.clone()) { continue; }
            self.postings.entry(token).or_default().push(name.to_string());
        }
        
        // エイリアス（別名）もインデックス化
        for token in aliases {
            if !seen.insert(token.clone()) { continue; }
            self.postings.entry(token.clone()).or_default().push(name.to_string());
            
            // エイリアスも2-gramトークン化（部分一致用）
            let alias_tokens = tokenize_2gram(&token);
            for subtoken in alias_tokens {
                if !seen.insert(subtoken.clone()) { continue; }
                self.postings.entry(subtoken).or_default().push(name.to_string());
            }
        }
        
        Ok(())
    }

    #[wasm_bindgen(js_name = "updateDocument")]
    pub fn update_document(&mut self, doc_id: &str, aliases_json: &str) -> Result<bool, JsValue> {
        // ドキュメントが存在するか確認
        if !self.doc_len.contains_key(doc_id) {
            return Ok(false);
        }
        
        // 古いドキュメントを削除して、新しいドキュメントを追加
        self.remove_doc(doc_id.to_string());
        self.add_document(doc_id, aliases_json)?;
        
        Ok(true)
    }

    #[wasm_bindgen(js_name = "replaceAllDocuments")]
    pub fn replace_all_documents(&mut self, json: &str) -> Result<(), JsValue> {
        // 現在のインデックスをクリア
        self.postings.clear();
        self.doc_len.clear();
        self.n_docs = 0;
        
        // 新しいドキュメントを追加
        self.add_documents(json)
    }

    #[wasm_bindgen(js_name = "clearIndex")]
    pub fn clear_index(&mut self) {
        self.postings.clear();
        self.doc_len.clear();
        self.n_docs = 0;
    }
}
