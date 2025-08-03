use std::sync::Arc;
use rustc_hash::FxHashMap as HashMap;
use wana_kana::ConvertJapanese;

/// 文字列キャッシュを管理する構造体
#[derive(Default)]
pub struct StringCache {
    /// 小文字変換のキャッシュ
    pub lowercase_cache: HashMap<Arc<String>, Arc<String>>,
    /// ひらがな変換のキャッシュ
    pub hiragana_cache: HashMap<Arc<String>, Arc<String>>,
    /// エイリアスから文書名への逆引きインデックス
    pub alias_to_doc: HashMap<Arc<String>, Vec<Arc<String>>>,
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