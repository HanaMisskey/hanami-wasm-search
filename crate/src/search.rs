use std::sync::Arc;
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use wana_kana::ConvertJapanese;

use crate::cache::{StringCache, MatchPriority};

/// 検索エンジンの実装
pub struct SearchEngine<'a> {
    pub doc_aliases: &'a HashMap<Arc<String>, Vec<Arc<String>>>,
    pub cache: &'a mut StringCache,
}

impl<'a> SearchEngine<'a> {


    /// AND検索の実装
    pub fn search_and(&mut self, keywords: Vec<&str>, limit: usize) -> Vec<String> {
        let mut matches = Vec::with_capacity(limit);
        let mut seen = HashSet::with_capacity_and_hasher(limit, Default::default());

        // 名前にすべてのキーワードが含まれている
        for (doc_name, _) in self.doc_aliases.iter() {
            let doc_name_lower = self.cache.get_lowercase(doc_name);
            let doc_name_hiragana = self.cache.get_hiragana(doc_name);
            
            if keywords.iter().all(|keyword| {
                doc_name_lower.contains(keyword) || 
                doc_name_hiragana.as_ref()
                    .map_or(false, |h| h.contains(&keyword.to_hiragana()))
            }) {
                if seen.insert(Arc::clone(doc_name)) {
                    matches.push(Arc::clone(doc_name));
                    if matches.len() >= limit {
                        return matches.into_iter().map(|arc| (*arc).clone()).collect();
                    }
                }
            }
        }
        
        // 名前またはエイリアスにすべてのキーワードが含まれている
        for (doc_name, aliases) in self.doc_aliases.iter() {
            if seen.contains(doc_name) {
                continue;
            }

            let doc_name_lower = self.cache.get_lowercase(doc_name);
            let doc_name_hiragana = self.cache.get_hiragana(doc_name);
            
            if keywords.iter().all(|keyword| {
                // 名前のチェック
                if doc_name_lower.contains(keyword) || 
                   doc_name_hiragana.as_ref()
                       .map_or(false, |h| h.contains(&keyword.to_hiragana())) {
                    return true;
                }
                
                // エイリアスのチェック
                aliases.iter().any(|alias| {
                    let alias_lower = self.cache.get_lowercase(alias);
                    let alias_hiragana = self.cache.get_hiragana(alias);
                    
                    alias_lower.contains(keyword) || 
                    alias_hiragana.as_ref()
                        .map_or(false, |h| h.contains(&keyword.to_hiragana()))
                })
            }) {
                if seen.insert(Arc::clone(doc_name)) {
                    matches.push(Arc::clone(doc_name));
                    if matches.len() >= limit {
                        return matches.into_iter().map(|arc| (*arc).clone()).collect();
                    }
                }
            }
        }

        matches.into_iter().map(|arc| (*arc).clone()).collect()
    }

    /// 優先度ベースの統合検索
    pub fn search_unified(&mut self, queries: &[String], limit: usize) -> Vec<String> {
        let mut candidates: Vec<(MatchPriority, Arc<String>)> = Vec::new();
        let mut seen = HashSet::with_capacity_and_hasher(limit * 2, Default::default());

        for (doc_name, aliases) in self.doc_aliases.iter() {
            if seen.contains(doc_name) {
                continue;
            }
            
            let mut best_priority = None;
            
            for query in queries {
                let doc_lower = self.cache.get_lowercase(doc_name);
                let doc_hiragana = self.cache.get_hiragana(doc_name);
                
                // 1. 名前の完全一致
                if doc_lower.as_str() == query {
                    best_priority = Some(MatchPriority::NameExact);
                    break; // 最高優先度なので即座に終了
                }
                
                // Romajiからひらがなに変換した場合の完全一致もチェック
                let query_hiragana = query.to_hiragana();
                if doc_lower.as_str() == &query_hiragana {
                    best_priority = Some(MatchPriority::NameExact);
                    break;
                }
                
                // 3. 名前の前方一致
                if best_priority.map_or(true, |p| p > MatchPriority::NamePrefix) {
                    if doc_lower.starts_with(query) || doc_lower.starts_with(&query_hiragana) {
                        best_priority = Some(MatchPriority::NamePrefix);
                    }
                }
                
                // 5. 名前の部分一致（ひらがな変換含む）
                if best_priority.map_or(true, |p| p > MatchPriority::NamePartial) {
                    if doc_lower.contains(query) || doc_lower.contains(&query_hiragana) {
                        best_priority = Some(MatchPriority::NamePartial);
                    } else if let Some(hiragana) = &doc_hiragana {
                        if hiragana.contains(&query.to_hiragana()) {
                            best_priority = Some(MatchPriority::NamePartial);
                        }
                    }
                }
                
                // エイリアスのチェック（名前の完全一致でない場合のみ）
                if best_priority != Some(MatchPriority::NameExact) {
                    for alias in aliases {
                        let alias_lower = self.cache.get_lowercase(alias);
                        let alias_hiragana = self.cache.get_hiragana(alias);
                        
                        // 2. エイリアスの完全一致
                        if alias_lower.as_str() == query && 
                           best_priority.map_or(true, |p| p > MatchPriority::AliasExact) {
                            best_priority = Some(MatchPriority::AliasExact);
                        }
                        // Romajiからひらがなに変換した場合のエイリアス完全一致もチェック
                        else if alias_lower.as_str() == &query_hiragana && 
                                best_priority.map_or(true, |p| p > MatchPriority::AliasExact) {
                            best_priority = Some(MatchPriority::AliasExact);
                        }
                        // 4. エイリアスの前方一致
                        else if (alias_lower.starts_with(query) || alias_lower.starts_with(&query_hiragana)) && 
                                best_priority.map_or(true, |p| p > MatchPriority::AliasPrefix) {
                            best_priority = Some(MatchPriority::AliasPrefix);
                        }
                        // 6. エイリアスの部分一致（ひらがな変換含む）
                        else if best_priority.map_or(true, |p| p >= MatchPriority::AliasPartial) {
                            if alias_lower.contains(query) {
                                best_priority = Some(MatchPriority::AliasPartial);
                            } else if alias_lower.contains(&query_hiragana) {
                                // ローマ字クエリをひらがなに変換して直接比較
                                best_priority = Some(MatchPriority::AliasPartial);
                            } else if let Some(hiragana) = &alias_hiragana {
                                if hiragana.contains(&query.to_hiragana()) {
                                    best_priority = Some(MatchPriority::AliasPartial);
                                }
                            }
                        }
                    }
                }
            }
            
            if let Some(priority) = best_priority {
                seen.insert(Arc::clone(doc_name));
                candidates.push((priority, Arc::clone(doc_name)));
                if candidates.len() >= limit * 2 {
                    break; // 十分な候補が集まったら終了
                }
            }
        }

        // 優先度でソートして結果を返す
        candidates.sort_by_key(|(p, _)| *p);
        candidates.into_iter()
            .take(limit)
            .map(|(_, name)| (*name).clone())
            .collect()
    }
}