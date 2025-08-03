use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use rustc_hash::FxHashMap as HashMap;
use std::sync::Arc;

mod cache;
mod search;

use cache::StringCache;
use search::SearchEngine;

// Helper module for Arc<String> serialization
mod arc_string_serde {
    use super::*;
    use serde::{Serializer, Deserializer};
    
    pub fn serialize<S>(map: &HashMap<Arc<String>, Vec<Arc<String>>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map_ser = serializer.serialize_map(Some(map.len()))?;
        for (k, v) in map {
            let v_strings: Vec<String> = v.iter().map(|s| (**s).clone()).collect();
            map_ser.serialize_entry(&**k, &v_strings)?;
        }
        map_ser.end()
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<Arc<String>, Vec<Arc<String>>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string_map: HashMap<String, Vec<String>> = HashMap::deserialize(deserializer)?;
        Ok(string_map.into_iter()
            .map(|(k, v)| (Arc::new(k), v.into_iter().map(Arc::new).collect()))
            .collect())
    }
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

#[wasm_bindgen]
#[derive(Serialize, Deserialize)]
pub struct Index {
    #[serde(with = "arc_string_serde")]
    doc_aliases: HashMap<Arc<String>, Vec<Arc<String>>>,
    n_docs: usize,
    #[serde(default = "default_version")]
    version: u32,
    #[serde(skip)]
    cache: StringCache,
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
            cache: StringCache::new(),
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
            let doc_name = Arc::new(doc.name);
            let aliases: Vec<Arc<String>> = doc.aliases.into_iter()
                .map(Arc::new)
                .collect();
            
            if self.doc_aliases.contains_key(&doc_name) { 
                self.remove_doc(doc_name.as_ref().clone()); 
            }
            
            self.doc_aliases.insert(Arc::clone(&doc_name), aliases);
            self.n_docs += 1;
        }
        
        // キャッシュを再構築
        self.rebuild_cache();
        
        Ok(())
    }

    #[wasm_bindgen(js_name = "search")]
    pub fn search(&mut self, query_json: &str, limit: Option<usize>) -> Result<JsValue, JsValue> {
        // JSONデシリアライズ
        let original: Vec<String> = match serde_json::from_str(query_json) {
            Ok(data) => data,
            Err(e) => return Err(JsValue::from_str(&e.to_string())),
        };
        
        let result_limit = limit.unwrap_or(10);
        
        if self.n_docs == 0 {
            return Ok(serde_wasm_bindgen::to_value(&Vec::<String>::new()).unwrap());
        }
        
        // クエリを小文字に変換
        let queries: Vec<String> = original.iter().map(|q| q.to_lowercase()).collect();
        
        // 検索エンジンを初期化
        let mut engine = SearchEngine {
            doc_aliases: &self.doc_aliases,
            cache: &mut self.cache,
        };
        
        // 単一クエリの早期終了最適化は一時的に無効化
        // (romaji-to-hiragana変換に対応していないため)
        
        // AND検索（スペース区切り）
        if queries.len() == 1 && queries[0].contains(' ') {
            let keywords: Vec<&str> = queries[0].split(' ').collect();
            let results = engine.search_and(keywords, result_limit);
            return Ok(serde_wasm_bindgen::to_value(&results).unwrap());
        }
        
        // 優先度ベースの統合検索
        let results = engine.search_unified(&queries, result_limit);
        Ok(serde_wasm_bindgen::to_value(&results).unwrap())
    }

    #[wasm_bindgen(js_name = "searchNoLimit")]
    pub fn search_no_limit(&mut self, query_json: &str) -> Result<JsValue, JsValue> {
        // 元の関数をラップし、制限なし（None）で呼び出す
        self.search(query_json, None)
    }

    #[wasm_bindgen(js_name = "searchWithLimit")]
    pub fn search_with_limit(&mut self, query_json: &str, limit: usize) -> Result<JsValue, JsValue> {
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
            Ok(mut index) => {
                // キャッシュを再構築
                index.rebuild_cache();
                Ok(index)
            },
            Err(_) => {
                // 失敗したら旧形式として読み込みを試みる
                match bincode::deserialize::<OldIndex>(&bytes_vec) {
                    Ok(old_index) => {
                        // 旧形式から新形式へマイグレーション
                        let mut index = Index {
                            doc_aliases: old_index.doc_aliases.into_iter()
                                .map(|(k, v)| {
                                    (Arc::new(k), v.into_iter().map(Arc::new).collect())
                                })
                                .collect(),
                            n_docs: old_index.n_docs,
                            version: 2,
                            cache: StringCache::new(),
                        };
                        // キャッシュを再構築
                        index.rebuild_cache();
                        Ok(index)
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
        let doc_id_arc = Arc::new(doc_id);
        if let Some(aliases) = self.doc_aliases.remove(&doc_id_arc) {
            // キャッシュから削除
            self.cache.remove_document(&doc_id_arc, &aliases);
            self.n_docs = self.n_docs.saturating_sub(1);
        }
    }

    #[wasm_bindgen(js_name = "removeDocument")]
    pub fn remove_document(&mut self, doc_id: &str) -> Result<bool, JsValue> {
        let doc_id_arc = Arc::new(doc_id.to_string());
        if self.doc_aliases.contains_key(&doc_id_arc) {
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
        
        let doc_name = Arc::new(name.to_string());
        let arc_aliases: Vec<Arc<String>> = aliases.into_iter()
            .map(Arc::new)
            .collect();
        
        // 既存のドキュメントなら削除
        if self.doc_aliases.contains_key(&doc_name) { 
            self.remove_doc(name.to_string()); 
        }
        
        // キャッシュを更新
        self.update_cache_for_document(&doc_name, &arc_aliases);
        
        self.doc_aliases.insert(doc_name, arc_aliases);
        self.n_docs += 1;
        
        Ok(())
    }

    #[wasm_bindgen(js_name = "updateDocument")]
    pub fn update_document(&mut self, doc_id: &str, aliases_json: &str) -> Result<bool, JsValue> {
        let doc_id_arc = Arc::new(doc_id.to_string());
        
        // ドキュメントが存在するか確認
        if !self.doc_aliases.contains_key(&doc_id_arc) {
            return Ok(false);
        }
        
        // エイリアスの検証
        let _: Vec<String> = serde_json::from_str(aliases_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        // アップデート前のドキュメントを削除
        self.remove_doc(doc_id.to_string());
        
        // 新しいドキュメントを追加
        self.add_document(doc_id, aliases_json)?;
        
        Ok(true)
    }

    #[wasm_bindgen(js_name = "replaceAllDocuments")]
    pub fn replace_all_documents(&mut self, json: &str) -> Result<(), JsValue> {
        // 現在のインデックスをクリア
        self.doc_aliases.clear();
        self.n_docs = 0;
        self.cache.clear();
        
        // 新しいドキュメントを追加
        self.add_documents(json)
    }

    #[wasm_bindgen(js_name = "clearIndex")]
    pub fn clear_index(&mut self) {
        self.doc_aliases.clear();
        self.n_docs = 0;
        self.cache.clear();
    }
    
    #[wasm_bindgen(js_name = "getVersion")]
    pub fn get_version(&self) -> u32 {
        self.version
    }
    
    // 内部メソッド（非公開）
    
    /// キャッシュを再構築
    fn rebuild_cache(&mut self) {
        self.cache.clear();
        
        for (doc_name, aliases) in &self.doc_aliases {
            // ドキュメント名のキャッシュを構築
            self.cache.get_lowercase(doc_name);
            self.cache.get_hiragana(doc_name);
            
            // エイリアスのキャッシュと逆引きインデックスを構築
            for alias in aliases {
                self.cache.get_lowercase(alias);
                self.cache.get_hiragana(alias);
                self.cache.add_alias_mapping(Arc::clone(alias), Arc::clone(doc_name));
            }
        }
    }
    
    /// 単一ドキュメントのキャッシュを更新
    fn update_cache_for_document(&mut self, doc_name: &Arc<String>, aliases: &[Arc<String>]) {
        // ドキュメント名のキャッシュを追加
        self.cache.get_lowercase(doc_name);
        self.cache.get_hiragana(doc_name);
        
        // エイリアスのキャッシュと逆引きインデックスを追加
        for alias in aliases {
            self.cache.get_lowercase(alias);
            self.cache.get_hiragana(alias);
            self.cache.add_alias_mapping(Arc::clone(alias), Arc::clone(doc_name));
        }
    }
}
