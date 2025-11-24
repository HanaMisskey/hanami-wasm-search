use std::sync::Arc;
use rustc_hash::FxHashMap as HashMap;
use wana_kana::ConvertJapanese;
use fst::{Map, MapBuilder};

/// 文字列キャッシュを管理する構造体
#[derive(Default)]
pub struct StringCache {
    /// 小文字変換のキャッシュ
    pub lowercase_cache: HashMap<Arc<String>, Arc<String>>,
    /// ひらがな変換のキャッシュ
    pub hiragana_cache: HashMap<Arc<String>, Arc<String>>,
    /// エイリアスから文書名への逆引きインデックス
    pub alias_to_doc: HashMap<Arc<String>, Vec<Arc<String>>>,
    /// 正規化キー→postings index（完全一致 O(1)）
    pub key_to_posting: HashMap<String, usize>,
    /// postings 本体（キーごとの doc と name/alias 判定）
    pub postings: Vec<Vec<Posting>>,
    /// FST（プレフィックス/ファジー探索用）
    pub fst_map: Option<Map<Vec<u8>>>,
}

/// 投稿（キーに紐づくドキュメントと種別）
#[derive(Clone)]
pub struct Posting {
    pub doc: Arc<String>,
    pub is_name: bool,
}

impl StringCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// 文字列の小文字変換をキャッシュから取得または生成
    pub fn get_lowercase(&mut self, text: &Arc<String>) -> Arc<String> {
        self.lowercase_cache
            .entry(Arc::clone(text))
            .or_insert_with(|| Arc::new(text.to_lowercase()))
            .clone()
    }

    /// 文字列のひらがな変換をキャッシュから取得または生成
    pub fn get_hiragana(&mut self, text: &Arc<String>) -> Option<Arc<String>> {
        // ASCII文字のみの場合、ひらがな変換を実行
        if text.chars().all(|c| c.is_ascii()) {
            Some(
                self.hiragana_cache
                    .entry(Arc::clone(text))
                    .or_insert_with(|| Arc::new(text.to_lowercase().to_hiragana()))
                    .clone()
            )
        } else {
            None
        }
    }

    /// キャッシュをクリア
    pub fn clear(&mut self) {
        self.lowercase_cache.clear();
        self.hiragana_cache.clear();
        self.alias_to_doc.clear();
        self.key_to_posting.clear();
        self.postings.clear();
        self.fst_map = None;
    }

    /// エイリアスの逆引きインデックスに追加
    pub fn add_alias_mapping(&mut self, alias: Arc<String>, doc_name: Arc<String>) {
        self.alias_to_doc
            .entry(alias)
            .or_insert_with(Vec::new)
            .push(doc_name);
    }

    /// エイリアスの逆引きインデックスから削除
    pub fn remove_alias_mapping(&mut self, alias: &str, doc_name: &str) {
        // Create temporary Arc for lookup
        let alias_arc = Arc::new(alias.to_string());
        if let Some(docs) = self.alias_to_doc.get_mut(&alias_arc) {
            docs.retain(|d| d.as_str() != doc_name);
        }
        // Check if empty and remove if necessary
        if self.alias_to_doc.get(&alias_arc).map_or(false, |docs| docs.is_empty()) {
            self.alias_to_doc.remove(&alias_arc);
        }
    }

    /// 特定のドキュメントに関連するキャッシュエントリを削除
    pub fn remove_document(&mut self, doc_name: &Arc<String>, aliases: &[Arc<String>]) {
        // 小文字・ひらがなキャッシュから削除
        self.lowercase_cache.remove(doc_name);
        self.hiragana_cache.remove(doc_name);
        
        // エイリアスのキャッシュも削除
        for alias in aliases {
            self.lowercase_cache.remove(alias);
            self.hiragana_cache.remove(alias);
            self.remove_alias_mapping(alias.as_str(), doc_name.as_str());
        }
    }

    /// 正規化済みキーのリストを取得（lowercase + hiragana 変換）
    pub fn normalized_keys(&mut self, text: &Arc<String>) -> Vec<String> {
        let mut keys = Vec::with_capacity(2);

        let lower = self.get_lowercase(text);
        keys.push((*lower).clone());

        if let Some(hira) = self.get_hiragana(text) {
            let hira_str = (*hira).clone();
            if hira_str != *lower {
                keys.push(hira_str);
            }
        }

        keys
    }

    /// doc_aliases から FST と postings を再構築
    pub fn rebuild_lexicon(&mut self, doc_aliases: &HashMap<Arc<String>, Vec<Arc<String>>>) -> Result<(), fst::Error> {
        // 前回のキャッシュを初期化
        self.key_to_posting.clear();
        self.postings.clear();
        self.fst_map = None;
        self.alias_to_doc.clear();

        // 一旦キー -> (doc, is_name) のリストを作る
        let mut temp: HashMap<String, Vec<(Arc<String>, bool)>> = HashMap::default();

        for (doc_name, aliases) in doc_aliases {
            // ドキュメント名
            for key in self.normalized_keys(doc_name) {
                temp.entry(key).or_default().push((Arc::clone(doc_name), true));
            }

            // エイリアス
            for alias in aliases {
                for key in self.normalized_keys(alias) {
                    temp.entry(key).or_default().push((Arc::clone(doc_name), false));
                }
                // 逆引き用マップも更新
                self.add_alias_mapping(Arc::clone(alias), Arc::clone(doc_name));
            }
        }

        // キーをソートして FST を構築
        let mut keys: Vec<String> = temp.keys().cloned().collect();
        keys.sort_unstable();

        let mut postings: Vec<Vec<Posting>> = Vec::with_capacity(keys.len());
        let mut builder = MapBuilder::memory();

        for (idx, key) in keys.iter().enumerate() {
            if let Some(entries) = temp.get(key) {
                // 同じ doc の重複を統合しつつ name/alias を保持
                let mut merged: HashMap<Arc<String>, bool> = HashMap::default();
                for (doc, is_name) in entries {
                    merged.entry(Arc::clone(doc))
                        .and_modify(|flag| *flag |= *is_name)
                        .or_insert(*is_name);
                }

                let mut posting_vec: Vec<Posting> = merged.into_iter()
                    .map(|(doc, is_name)| Posting { doc, is_name })
                    .collect();

                // 安定した順序にするため doc 名でソート
                posting_vec.sort_by(|a, b| a.doc.cmp(&b.doc));

                postings.push(posting_vec);
            } else {
                postings.push(Vec::new());
            }
            builder.insert(key, idx as u64)?;
        }

        let fst_bytes = builder.into_inner()?;
        self.fst_map = Some(Map::new(fst_bytes)?);
        self.key_to_posting = keys.into_iter()
            .enumerate()
            .map(|(i, k)| (k, i))
            .collect();
        self.postings = postings;

        Ok(())
    }
}

/// 検索時の優先度を表す列挙型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchPriority {
    NameExact = 1,
    AliasExact = 2,
    NamePrefix = 3,
    AliasPrefix = 4,
    NamePartial = 5,
    AliasPartial = 6,
}
