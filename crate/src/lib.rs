use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use wana_kana::ConvertJapanese;

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

#[wasm_bindgen]
#[derive(Serialize, Deserialize)]
pub struct Index {
    doc_aliases: HashMap<String, Vec<String>>,
    n_docs: usize,
    #[serde(default = "default_version")]
    version: u32,
}

fn default_version() -> u32 {
    2  // Current version
}

// 旧バージョンのIndex構造体（マイグレーション用）
#[derive(Deserialize)]
#[allow(dead_code)]
struct OldIndex {
    postings: HashMap<String, Vec<String>>,
    doc_len: HashMap<String, usize>,
    doc_aliases: HashMap<String, Vec<String>>,
    n_docs: usize,
    k1: f32,
    b: f32,
    #[serde(default)]
    version: u32,
}

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
    
    format!("{}\nContext: '{}'", error_msg, context)
}

#[wasm_bindgen]
impl Index {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Index {
        Index { 
            doc_aliases: HashMap::default(),
            n_docs: 0,
            version: 2,
        }
    }

    pub fn add_documents(&mut self, json: &str) -> Result<(), JsValue> {
        let data: EmojisData = match serde_json::from_str(json) {
            Ok(data) => data,
            Err(e) => {
                return Err(JsValue::from_str(&log_json_error(json, &e)));
            }
        };
        
        let emoji_count = data.emojis.len();
        
        // 事前確保
        if self.doc_aliases.is_empty() {
            self.doc_aliases = HashMap::with_capacity_and_hasher(emoji_count, Default::default());
        }
        
        for doc in data.emojis {
            let doc_name = doc.name.clone();
            
            if self.doc_aliases.contains_key(&doc_name) { 
                self.remove_doc(doc_name.clone()); 
            }
            
            self.doc_aliases.insert(doc_name.clone(), doc.aliases);
            self.n_docs += 1;
        }
        
        Ok(())
    }

    #[wasm_bindgen(js_name = "search")]
    pub fn search(&self, query_json: &str, limit: Option<usize>) -> Result<JsValue, JsValue> {
        // JSONデシリアライズ
        let original: Vec<String> = match serde_json::from_str(query_json) {
            Ok(data) => data,
            Err(e) => return Err(JsValue::from_str(&e.to_string())),
        };
        
        let result_limit = limit.unwrap_or(10);
        
        if self.n_docs == 0 {
            return Ok(serde_wasm_bindgen::to_value(&Vec::<String>::new()).unwrap());
        }
        
        // 結果を格納するセット（重複を避けるため）
        let mut matches = Vec::with_capacity(result_limit);
        let mut seen = HashSet::with_capacity_and_hasher(result_limit, Default::default());
        
        // クエリを小文字に変換
        let queries: Vec<String> = original.iter().map(|q| q.to_lowercase()).collect();
        
        // 全ドキュメントをイテレート（より愚直なアプローチ）
        let all_docs: Vec<(&String, &Vec<String>)> = self.doc_aliases.iter().collect();
        
        // 1. 完全一致チェック（名前）
        for query in &queries {
            if let Some(_aliases) = self.doc_aliases.get(query) {
                if seen.insert(query.clone()) {
                    matches.push(query.clone());
                    if matches.len() >= result_limit {
                        return Ok(serde_wasm_bindgen::to_value(&matches).unwrap());
                    }
                }
            }
        }
        
        // AND検索（スペース区切り）
        if queries.len() == 1 && queries[0].contains(' ') {
            let keywords: Vec<&str> = queries[0].split(' ').collect();
            
            // 名前にすべてのキーワードが含まれている
            for (doc_name, _) in &all_docs {
                let doc_name_lower = doc_name.to_lowercase();
                if keywords.iter().all(|keyword| {
                    doc_name_lower.contains(keyword) || 
                    doc_name_lower.to_hiragana().contains(&keyword.to_hiragana())
                }) {
                    if seen.insert((*doc_name).clone()) {
                        matches.push((*doc_name).clone());
                        if matches.len() >= result_limit {
                            return Ok(serde_wasm_bindgen::to_value(&matches).unwrap());
                        }
                    }
                }
            }
            
            // 名前またはエイリアスにすべてのキーワードが含まれている
            for (doc_name, aliases) in &all_docs {
                let doc_name_lower = doc_name.to_lowercase();
                if keywords.iter().all(|keyword| {
                    doc_name_lower.contains(keyword) || 
                    doc_name_lower.to_hiragana().contains(&keyword.to_hiragana()) ||
                    aliases.iter().any(|alias| {
                        let alias_lower = alias.to_lowercase();
                        alias_lower.contains(keyword) || 
                        alias_lower.to_hiragana().contains(&keyword.to_hiragana())
                    })
                }) {
                    if seen.insert((*doc_name).clone()) {
                        matches.push((*doc_name).clone());
                        if matches.len() >= result_limit {
                            return Ok(serde_wasm_bindgen::to_value(&matches).unwrap());
                        }
                    }
                }
            }
        } else {
            // 単一キーワード検索
            for query in &queries {
                // 2. エイリアスとの完全一致
                for (doc_name, aliases) in &all_docs {
                    if aliases.iter().any(|alias| alias.to_lowercase() == *query) {
                        if seen.insert((*doc_name).clone()) {
                            matches.push((*doc_name).clone());
                            if matches.len() >= result_limit {
                                return Ok(serde_wasm_bindgen::to_value(&matches).unwrap());
                            }
                        }
                    }
                }
                
                // 3. 名前が検索語で始まる
                for (doc_name, _) in &all_docs {
                    if doc_name.to_lowercase().starts_with(query) {
                        if seen.insert((*doc_name).clone()) {
                            matches.push((*doc_name).clone());
                            if matches.len() >= result_limit {
                                return Ok(serde_wasm_bindgen::to_value(&matches).unwrap());
                            }
                        }
                    }
                }
                
                // 4. エイリアスが検索語で始まる
                for (doc_name, aliases) in &all_docs {
                    if aliases.iter().any(|alias| alias.to_lowercase().starts_with(query)) {
                        if seen.insert((*doc_name).clone()) {
                            matches.push((*doc_name).clone());
                            if matches.len() >= result_limit {
                                return Ok(serde_wasm_bindgen::to_value(&matches).unwrap());
                            }
                        }
                    }
                }
                
                // 5. 名前に検索語が含まれる（ひらがな変換含む）
                for (doc_name, _) in &all_docs {
                    let doc_name_lower = doc_name.to_lowercase();
                    if doc_name_lower.contains(query) || 
                       doc_name_lower.to_hiragana().contains(&query.to_hiragana()) {
                        if seen.insert((*doc_name).clone()) {
                            matches.push((*doc_name).clone());
                            if matches.len() >= result_limit {
                                return Ok(serde_wasm_bindgen::to_value(&matches).unwrap());
                            }
                        }
                    }
                }
                
                // 6. エイリアスに検索語が含まれる（ひらがな変換含む）
                for (doc_name, aliases) in &all_docs {
                    if aliases.iter().any(|alias| {
                        let alias_lower = alias.to_lowercase();
                        alias_lower.contains(query) || 
                        alias_lower.to_hiragana().contains(&query.to_hiragana())
                    }) {
                        if seen.insert((*doc_name).clone()) {
                            matches.push((*doc_name).clone());
                            if matches.len() >= result_limit {
                                return Ok(serde_wasm_bindgen::to_value(&matches).unwrap());
                            }
                        }
                    }
                }
            }
        }
        
        Ok(serde_wasm_bindgen::to_value(&matches).unwrap())
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
        let bytes_vec = bytes.to_vec();
        
        // まず新しい形式で読み込みを試みる
        match bincode::deserialize::<Index>(&bytes_vec) {
            Ok(index) => Ok(index),
            Err(_) => {
                // 失敗したら旧形式として読み込みを試みる
                match bincode::deserialize::<OldIndex>(&bytes_vec) {
                    Ok(old_index) => {
                        // 旧形式から新形式へマイグレーション
                        Ok(Index {
                            doc_aliases: old_index.doc_aliases,
                            n_docs: old_index.n_docs,
                            version: 2,
                        })
                    }
                    Err(e) => Err(JsValue::from_str(&format!(
                        "Failed to load index: {}. The index format may be incompatible.",
                        e
                    )))
                }
            }
        }
    }

    fn remove_doc(&mut self, doc_id: String) {
        if self.doc_aliases.remove(&doc_id).is_some() {
            self.n_docs = self.n_docs.saturating_sub(1);
        }
    }

    #[wasm_bindgen(js_name = "removeDocument")]
    pub fn remove_document(&mut self, doc_id: &str) -> Result<bool, JsValue> {
        if self.doc_aliases.contains_key(doc_id) {
            self.remove_doc(doc_id.to_owned());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    #[wasm_bindgen(js_name = "addDocument")]
    pub fn add_document(&mut self, name: &str, aliases_json: &str) -> Result<(), JsValue> {
        let aliases: Vec<String> = serde_json::from_str(aliases_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        let doc_name = name.to_string();
        
        // 既存のドキュメントなら削除
        if self.doc_aliases.contains_key(&doc_name) { 
            self.remove_doc(doc_name.clone()); 
        }
        
        self.doc_aliases.insert(doc_name, aliases);
        self.n_docs += 1;
        
        Ok(())
    }

    #[wasm_bindgen(js_name = "updateDocument")]
    pub fn update_document(&mut self, doc_id: &str, aliases_json: &str) -> Result<bool, JsValue> {
        // ドキュメントが存在するか確認
        if !self.doc_aliases.contains_key(doc_id) {
            return Ok(false);
        }
        
        // エイリアスの検証
        match serde_json::from_str::<Vec<String>>(aliases_json) {
            Ok(_) => (), // Just checking validity
            Err(e) => return Err(JsValue::from_str(&e.to_string())),
        };
        
        // アップデート前のドキュメントを削除
        self.remove_doc(doc_id.to_string());
        self.add_document(doc_id, aliases_json)?;
        
        Ok(true)
    }

    #[wasm_bindgen(js_name = "replaceAllDocuments")]
    pub fn replace_all_documents(&mut self, json: &str) -> Result<(), JsValue> {
        // 現在のインデックスをクリア
        self.doc_aliases.clear();
        self.n_docs = 0;
        
        // 新しいドキュメントを追加
        self.add_documents(json)
    }

    #[wasm_bindgen(js_name = "clearIndex")]
    pub fn clear_index(&mut self) {
        self.doc_aliases.clear();
        self.n_docs = 0;
    }
    
    #[wasm_bindgen(js_name = "getVersion")]
    pub fn get_version(&self) -> u32 {
        self.version
    }
}
