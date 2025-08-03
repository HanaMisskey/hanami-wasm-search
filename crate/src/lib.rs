use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use wana_kana::ConvertJapanese;

// マッチタイプのフラグを定義（ビットフラグ）
const MATCH_NAME_EXACT: u8 = 0b100000; // nameと完全一致
const MATCH_ALIAS_EXACT: u8 = 0b010000; // aliasと完全一致
const MATCH_NAME_PREFIX: u8 = 0b001000; // nameと前方一致
const MATCH_ALIAS_PREFIX: u8 = 0b000100; // aliasと前方一致
const MATCH_NAME_PARTIAL: u8 = 0b000010; // nameと部分一致
const MATCH_ALIAS_PARTIAL: u8 = 0b000001; // aliasと部分一致

// 優先度スコア定数
const PRIORITY_NAME_EXACT: u8 = 100;
const PRIORITY_ALIAS_EXACT: u8 = 90;
const PRIORITY_NAME_PREFIX: u8 = 80;
const PRIORITY_ALIAS_PREFIX: u8 = 70;
const PRIORITY_NAME_PARTIAL: u8 = 60;
const PRIORITY_ALIAS_PARTIAL: u8 = 50;

type Postings = HashMap<String, Vec<String>>;

// 2-gram（バイグラム）トークン化関数
fn tokenize_2gram(text: &str) -> Vec<String> {
    let char_count = text.chars().count();
    
    if char_count <= 1 {
        return if char_count == 0 {
            Vec::new()
        } else {
            vec![text.to_string()]
        };
    }
    
    // 必要な容量を正確に計算: 2-gram数 + 元の単語
    // 2-gram数 = char_count - 1, +1 for 元の単語
    let mut tokens = Vec::with_capacity(char_count);
    
    // 文字列の効率的なスライス処理のためcharインデックス位置を記録
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    
    // 2-gramトークンを生成 
    for i in 0..chars.len() - 1 {
        let (start_idx, _) = chars[i];
        let (end_idx, end_char) = chars[i + 1];
        // 文字境界を正確に判断し、バイトスライスを使う
        let token = text[start_idx..end_idx + end_char.len_utf8()].to_string();
        tokens.push(token);
    }
    
    // 元の単語自体も追加（完全一致検索のため）
    tokens.push(text.to_string());
    
    tokens
}

// 優先度を計算するヘルパー関数
fn calculate_priority(match_type: u8) -> u8 {
    if match_type & MATCH_NAME_EXACT != 0 { 
        PRIORITY_NAME_EXACT 
    } else if match_type & MATCH_ALIAS_EXACT != 0 { 
        PRIORITY_ALIAS_EXACT 
    } else if match_type & MATCH_NAME_PREFIX != 0 { 
        PRIORITY_NAME_PREFIX 
    } else if match_type & MATCH_ALIAS_PREFIX != 0 { 
        PRIORITY_ALIAS_PREFIX 
    } else if match_type & MATCH_NAME_PARTIAL != 0 { 
        PRIORITY_NAME_PARTIAL 
    } else if match_type & MATCH_ALIAS_PARTIAL != 0 { 
        PRIORITY_ALIAS_PARTIAL 
    } else { 
        0 
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
    postings: Postings,
    doc_len: HashMap<String, usize>,
    doc_aliases: HashMap<String, Vec<String>>,
    n_docs: usize,
    k1: f32,
    b: f32,
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
        Self::with_params(1.2, 0.75)
    }

    #[wasm_bindgen(js_name = "newWithParams")]
    pub fn with_params(k1: f32, b: f32) -> Index {
        Index { 
            postings: HashMap::default(), 
            doc_len: HashMap::default(), 
            doc_aliases: HashMap::default(),
            n_docs: 0, 
            k1, 
            b 
        }
    }

    pub fn set_params(&mut self, k1: f32, b: f32) { self.k1 = k1; self.b = b; }

    pub fn add_documents(&mut self, json: &str) -> Result<(), JsValue> {
        let data: EmojisData = match serde_json::from_str(json) {
            Ok(data) => data,
            Err(e) => {
                return Err(JsValue::from_str(&log_json_error(json, &e)));
            }
        };
        
        let emoji_count = data.emojis.len();
        
        // 事前確保
        if self.postings.is_empty() {
            let estimated_tokens = emoji_count * 20;
            self.postings = HashMap::with_capacity_and_hasher(estimated_tokens, Default::default());
            self.doc_len = HashMap::with_capacity_and_hasher(emoji_count, Default::default());
            self.doc_aliases = HashMap::with_capacity_and_hasher(emoji_count, Default::default());
        }
        
        for doc in data.emojis {
            let doc_name = doc.name.clone();
            
            if self.doc_len.contains_key(&doc_name) { self.remove_doc(doc_name.clone()); }
            
            // 名前も含めたトークン数を記録
            self.doc_len.insert(doc_name.clone(), doc.aliases.len() + 1);
            self.doc_aliases.insert(doc_name.clone(), doc.aliases.clone());
            self.n_docs += 1;
            
            // 推定トークン数から HashSet サイズを事前確保
            let estimated_total_tokens = 2 * (doc.name.len() + doc.aliases.iter().map(|a| a.len()).sum::<usize>());
            let mut seen = HashSet::with_capacity_and_hasher(estimated_total_tokens, Default::default());
            
            // 名前自体もインデックスに追加
            seen.insert(doc_name.clone());
            self.postings.entry(doc_name.clone()).or_default().push(doc_name.clone());
            
            // 名前の2-gramトークン化（部分一致用）
            let name_tokens = tokenize_2gram(&doc.name);
            let doc_name_ref = &doc_name;
            for token in name_tokens {
                if !seen.insert(token.clone()) { continue; }
                self.postings.entry(token).or_default().push(doc_name_ref.clone());
            }
            
            // エイリアス（別名）もインデックス化
            for alias in doc.aliases {
                if !seen.insert(alias.clone()) { continue; }
                self.postings.entry(alias.clone()).or_default().push(doc_name_ref.clone());
                
                let alias_tokens = tokenize_2gram(&alias);
                for subtoken in alias_tokens {
                    if !seen.insert(subtoken.clone()) { continue; }
                    self.postings.entry(subtoken).or_default().push(doc_name_ref.clone());
                }
            }
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
        
        // 平均ドキュメント長を事前計算
        let avg_len = self.doc_len.values().copied().sum::<usize>() as f32 / self.n_docs as f32;
        
        // クエリの長さを制限
        let max_term_len = original.iter().map(|s| s.chars().count()).max().unwrap_or(0);
        // 元の用語数 + 2-gram最大数 + ひらがな変換分
        let estimated_expanded_size = original.len() * (1 + max_term_len.saturating_sub(1) + 1);
        let mut expanded_2gram = Vec::with_capacity(estimated_expanded_size);
        let mut seen_tokens = HashSet::with_capacity_and_hasher(estimated_expanded_size, Default::default());
        
        let original_terms: HashSet<&str> = original.iter().map(|s| s.as_str()).collect();
        
        // クエリトークンがオリジナルから来たかどうかを判定するヘルパー関数
        fn token_is_from_original_query(token: &str, original_terms: &HashSet<&str>) -> bool {
            // クエリに完全一致するトークンがある場合
            if original_terms.contains(token) {
                return true;
            }
            
            // トークンがクエリのどれかの先頭になっている場合
            for &original in original_terms.iter() {
                if original.starts_with(token) {
                    return true;
                }
            }
            
            false
        }
        
        // クエリを2-gramに変換する
        for term in &original {
            // オリジナルの単語も検索対象に含める - ムーブで所有権を移動し、クローンを回避
            let term_str = term.as_str();
            expanded_2gram.push(term.clone());
            seen_tokens.insert(term_str.to_string());
            
            // メモリ使用量が少ない場合のみひらがな変換を行う
            if term_str.len() < 50 && term_str.chars().all(|c| c.is_ascii_alphabetic()) {
                let hira = term_str.to_lowercase().to_hiragana();
                if hira != *term {
                    let hira_str = hira.clone();
                    expanded_2gram.push(hira);
                    seen_tokens.insert(hira_str.clone());
                    
                    // 変換された日本語も2-gramトークン化
                    let hira_tokens = tokenize_2gram(&hira_str);
                    for token in hira_tokens {
                        if seen_tokens.insert(token.clone()) {
                            expanded_2gram.push(token);
                        }
                    }
                }
            }
            
            // 異常に長い単語の場合は2-gramを生成しない
            if term_str.len() < 100 {
                let tokens = tokenize_2gram(term_str);
                for token in tokens {
                    if seen_tokens.insert(token.clone()) {
                        expanded_2gram.push(token);
                    }
                }
            }
        }
        
        // 検索結果のサイズを推定してHashMapを初期化
        let estimated_results = (expanded_2gram.len().min(self.n_docs) / 2).max(10);
        let mut scores = HashMap::with_capacity_and_hasher(estimated_results, Default::default());
        let mut match_types = HashMap::with_capacity_and_hasher(estimated_results, Default::default());
        
        // 完全一致と部分一致を検出
        // 長いトークンから処理してより多くの関連ドキュメントを早期に検索
        // 重要な検索結果が優先され、後続の処理量を削減
        let mut expanded_2gram = expanded_2gram;
        expanded_2gram.sort_by(|a, b| b.len().cmp(&a.len()));
        
        // 検索結果制限を考慮して処理を最適化
        // 十分な候補を見つけたら早期終了
        let mut doc_count = 0;
        let early_exit_threshold = result_limit * 10; 
        
        for term in &expanded_2gram {
            if let Some(list) = self.postings.get(term) {
                let df = list.len() as f32;
                let idf = ((self.n_docs as f32 - df + 0.5)/(df+0.5) + 1.0).ln();
                
                // 原文クエリに対する検索
                let term_str = term.as_str();
                let original_contains_term = original_terms.contains(&term_str);
                
                for doc_id in list {
                    // 多すぎる候補を処理しないようにする
                    if doc_count > early_exit_threshold && scores.len() >= result_limit * 2 {
                        break;
                    }
                    doc_count += 1;
                    
                    // BM25
                    let tf = if original_contains_term { 2.0 } else { 1.0 };
                    let len = *self.doc_len.get(doc_id).unwrap_or(&1) as f32;
                    let score = idf * (tf*(self.k1+1.0)) /
                        (tf + self.k1*(1.0 - self.b + self.b*len/avg_len));
                    
                    // クエリと完全一致する場合のボーナス
                    let exact_match_bonus = if original.iter().any(|q| q == doc_id) {
                        10.0
                    } else {
                        0.0
                    };
                    
                    // クエリの完全包含チェック
                    let contains_full_query = original.iter().any(|q| {
                        doc_id.contains(q) || 
                        self.doc_aliases.get(doc_id)
                            .map(|aliases| aliases.iter().any(|a| a.contains(q)))
                            .unwrap_or(false)
                    });
                    
                    let containment_bonus = if contains_full_query {
                        5.0
                    } else {
                        0.0
                    };
                    
                    *scores.entry(doc_id.clone()).or_insert(0.0) += score + exact_match_bonus + containment_bonus;
                    
                    // マッチ種類を記録（ビットフラグを使用）
                    let entry = match_types.entry(doc_id.clone()).or_insert(0);
                    
                    // 完全一致の判定（最も頻度の高いケース）- 文字列比較を効率化
                    if original_contains_term {
                        if term_str == doc_id {
                            // nameと完全一致
                            *entry |= MATCH_NAME_EXACT;
                        } else {
                            // aliasとの完全一致チェック
                            *entry |= MATCH_ALIAS_EXACT;
                        }
                    } else {
                        // 前方一致判定を追加 - ドキュメント名が検索語で始まる場合
                        if term.len() <= 50 && doc_id.starts_with(term_str) {
                            *entry |= MATCH_NAME_PREFIX;
                        } 
                        // 部分一致の判定（より効率的なチェック） - 長い文字列は部分一致チェックを省略
                        else if term.len() <= 50 && doc_id.contains(term_str) {
                            *entry |= MATCH_NAME_PARTIAL;
                        } else {
                            // aliasとの前方一致または部分一致を区別して判断
                            // 注: 実際のaliasデータは検索中にはアクセスできないため、
                            // ポスティングリストの存在のみで判定する必要がある
                            
                            // トークンがキー自体の2-gramでない場合、前方一致または部分一致を判定
                            if token_is_from_original_query(term_str, &original_terms) {
                                // 部分一致と判断するのは2-gramの場合のみ
                                // 元のクエリからの単語は前方一致として扱う（より優先度が高い）
                                *entry |= MATCH_ALIAS_PREFIX;
                            } else if term.len() <= 2 {
                                // 短い2-gramは部分一致として扱う
                                // 2文字以下のバイグラムは部分一致の可能性が高い
                                *entry |= MATCH_ALIAS_PARTIAL;
                            }
                            // それ以外の場合はマークしない（部分一致でない）
                        }
                    }
                }
            }
        }
        
        // スコアリング結果の処理
        
        if scores.len() > result_limit * 2 && result_limit < 100 {
            // 少ない結果数でヒープソート
            use std::collections::BinaryHeap;
            use std::cmp::Reverse;
            
            // 優先度+スコアのヒープコンテナを使用して上位N件のみを保持
            #[derive(PartialEq, Eq)]
            struct ScoredDoc {
                priority: u8,
                // 浮動小数点比較のためにバイト表現に変換
                score_bits: u32,
                doc_id: String,
            }
            
            impl PartialOrd for ScoredDoc {
                fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                    Some(self.cmp(other))
                }
            }
            
            impl Ord for ScoredDoc {
                fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                    // 優先度が高いものが先に、優先度が同じなら高いスコアのものが先に
                    self.priority.cmp(&other.priority)
                        .then_with(|| self.score_bits.cmp(&other.score_bits))
                }
            }
            
            // 上位N件だけを保持
            let mut top_docs = BinaryHeap::with_capacity(result_limit + 1);
            
            // すべてのドキュメントをスコア計算して、ヒープに追加
            for (doc_id, score) in scores {
                let match_type = match_types.get(&doc_id).cloned().unwrap_or(0);
                
                // マッチタイプに基づいて優先度スコアを計算
                let priority = calculate_priority(match_type);
                
                // 浮動小数点を比較可能なビット表現に変換
                let score_bits = score.to_bits();
                
                // スコアドキュメントを作成（所有権移転で効率化）
                let scored_doc = ScoredDoc { priority, score_bits, doc_id };
                
                // MinHeapとして使うためReverseでラップ
                top_docs.push(Reverse(scored_doc));
                
                // ヒープサイズを制限
                if top_docs.len() > result_limit {
                    top_docs.pop();
                }
            }
            
            // 結果を取り出し
            let mut ranked = Vec::with_capacity(top_docs.len());
            while let Some(Reverse(doc)) = top_docs.pop() {
                ranked.push(doc.doc_id);
            }
            
            // ヒープから取り出した結果は昇順なので、降順に並べ替え
            ranked.reverse();
            
            Ok(serde_wasm_bindgen::to_value(&ranked).unwrap())
        } else {
            // 少数の結果
            let result_size = scores.len().min(result_limit);
            let mut results = Vec::with_capacity(result_size);
            
            // フィルタリングとスコアリング
            for (doc_id, score) in scores {
                let match_type = match_types.get(&doc_id).cloned().unwrap_or(0);
                results.push((doc_id, score, match_type));
            }
                
            // 優先順位付けでソート
            results.sort_by(|a, b| {
                let (_, a_score, a_match) = a;
                let (_, b_score, b_match) = b;
                
                // マッチタイプに基づいて優先度スコアを計算
                let a_priority = calculate_priority(*a_match);
                let b_priority = calculate_priority(*b_match);
                
                // 優先度で比較し、同じ場合はスコアで比較
                match b_priority.cmp(&a_priority) {
                    std::cmp::Ordering::Equal => b_score.partial_cmp(a_score).unwrap_or(std::cmp::Ordering::Equal),
                    other => other
                }
            });
            
            // 検索結果を制限して、IDのみを返す
            let mut ranked = Vec::with_capacity(result_size);
            for (id, _, _) in results.into_iter().take(result_limit) {
                ranked.push(id);
            }
            
            Ok(serde_wasm_bindgen::to_value(&ranked).unwrap())
        }
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
        // doc_lenが存在するか確認
        let doc_len = if let Some(len) = self.doc_len.remove(&doc_id) {
            len
        } else {
            return;
        };
        
        // エイリアス情報も削除
        self.doc_aliases.remove(&doc_id);
        
        // tokenize関数をこのスコープで使うため、id用のトークンを準備
        let mut tokens_to_check = Vec::with_capacity(doc_len + 2);
        
        // 最低限必要なトークンを追加（名前自体）
        tokens_to_check.push(doc_id.clone());
        
        // 部分一致用の2-gramトークンも追加（最も発生頻度が高いものだけ）
        // 文字数制限を設けることでメモリ使用量を抑制
        if doc_id.len() < 50 {
            for token in tokenize_2gram(&doc_id) {
                tokens_to_check.push(token);
            }
        }
        
        // 完全なスキャンの代わりに可能性の高いpostingだけをチェック
        for token in &tokens_to_check {
            if let Some(posting_list) = self.postings.get_mut(token) {
                // 所有権ベースの比較
                let before_len = posting_list.len();
                posting_list.retain(|id| id != &doc_id);
                
                // リストが空になった場合はエントリを削除
                if posting_list.is_empty() {
                    self.postings.remove(token);
                }
                // アイテムが削除されていない場合、このトークンに関連付けられたエイリアスは存在しない
                else if before_len == posting_list.len() && token == &doc_id {
                    // 名前自体のエントリに変更がない場合は、このドキュメントが他のトークンを持っていないと仮定
                    break;
                }
            }
        }
        
        // 見つからなかったポスティングリストをフォールバックとして処理
        self.postings.retain(|_, posting_list| {
            let before_len = posting_list.len();
            posting_list.retain(|id| id != &doc_id);
            // 変更がない場合は保持、空になった場合は削除
            posting_list.len() == before_len || !posting_list.is_empty()
        });
        
        self.n_docs = self.n_docs.saturating_sub(1);
    }

    #[wasm_bindgen(js_name = "removeDocument")]
    pub fn remove_document(&mut self, doc_id: &str) -> Result<bool, JsValue> {
        // 短い文字列の場合はto_ownedを使用
        // exists check + remove を一度の操作で行う
        if self.doc_len.contains_key(doc_id) {
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
        if self.doc_len.contains_key(&doc_name) { 
            self.remove_doc(doc_name.clone()); 
        }
        
        // 名前も含めたトークン数を記録
        self.doc_len.insert(doc_name.clone(), aliases.len() + 1); // +1 for name
        self.doc_aliases.insert(doc_name.clone(), aliases.clone());
        self.n_docs += 1;
        
        // 推定トークン数から HashSet サイズを事前確保
        let estimated_total_tokens = 2 * (name.len() + aliases.iter().map(|a| a.len()).sum::<usize>());
        let mut seen = HashSet::with_capacity_and_hasher(estimated_total_tokens, Default::default());
        
        // 名前自体のインデックス
        seen.insert(doc_name.clone());
        self.postings.entry(doc_name.clone()).or_default().push(doc_name.clone());
        // 名前の2-gramトークン
        let name_tokens = tokenize_2gram(name);
        let doc_name_ref = &doc_name;
        for token in name_tokens {
            if !seen.insert(token.clone()) { continue; }
            self.postings.entry(token).or_default().push(doc_name_ref.clone());
        }
        
        // エイリアスのインデックス
        for alias in aliases {
            if !seen.insert(alias.clone()) { continue; }
            self.postings.entry(alias.clone()).or_default().push(doc_name_ref.clone());            
            // エイリアスの2-gramトークン
            let alias_tokens = tokenize_2gram(&alias);
            for subtoken in alias_tokens {
                if !seen.insert(subtoken.clone()) { continue; }
                self.postings.entry(subtoken).or_default().push(doc_name_ref.clone());
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
        self.postings.clear();
        self.doc_len.clear();
        self.doc_aliases.clear();
        self.n_docs = 0;
        
        // 新しいドキュメントを追加
        self.add_documents(json)
    }

    #[wasm_bindgen(js_name = "clearIndex")]
    pub fn clear_index(&mut self) {
        self.postings.clear();
        self.doc_len.clear();
        self.doc_aliases.clear();
        self.n_docs = 0;
    }
}
