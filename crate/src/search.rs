use std::sync::Arc;
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use fst::{IntoStreamer, Streamer};
use wana_kana::ConvertJapanese;

use crate::cache::{StringCache, MatchPriority, Posting};

/// 検索エンジンの実装
pub struct SearchEngine<'a> {
    pub doc_aliases: &'a HashMap<Arc<String>, Vec<Arc<String>>>,
    pub cache: &'a mut StringCache,
}

/// マッチのスコープ（優先度決定用）
#[derive(Clone, Copy)]
enum Scope {
    Exact,
    Prefix,
    Partial,
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
        if limit == 0 {
            return Vec::new();
        }

        // doc ごとのベスト優先度を保持
        let mut best: HashMap<Arc<String>, MatchPriority> =
            HashMap::with_capacity_and_hasher(self.doc_aliases.len(), Default::default());
        let mut all_variants: Vec<String> = Vec::with_capacity(queries.len() * 2);

        for query in queries {
            let q_arc = Arc::new(query.clone());
            let variants = self.cache.normalized_keys(&q_arc);
            all_variants.extend(variants.iter().cloned());

            for key in variants {
                // 1) 完全一致: O(1) ルックアップ
                if let Some(idx) = self.cache.key_to_posting.get(&key) {
                    if let Some(postings) = self.cache.postings.get(*idx) {
                        self.record_hits(postings, Scope::Exact, &mut best);
                    }
                }

                // 2) 前方一致: FST range 検索
                self.collect_prefix(&key, &mut best);

                // 3) 軽いファジー (edit distance <= 1)
                self.collect_fuzzy(&key, &mut best);
            }
        }

        // 4) まだ枠に余裕がある場合は部分一致スキャン（小規模コーパス向け）
        if best.len() < limit {
            self.collect_partial(&all_variants, &mut best);
        }

        let mut candidates: Vec<(MatchPriority, Arc<String>)> = best.into_iter()
            .map(|(doc, prio)| (prio, doc))
            .collect();

        candidates.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        if candidates.len() > limit {
            candidates.truncate(limit);
        }

        candidates.into_iter()
            .map(|(_, doc)| (*doc).clone())
            .collect()
    }

    /// postings を優先度に応じて反映
    fn record_hits(
        &self,
        postings: &[Posting],
        scope: Scope,
        best: &mut HashMap<Arc<String>, MatchPriority>,
    ) {
        for p in postings {
            let prio = match (scope, p.is_name) {
                (Scope::Exact, true) => MatchPriority::NameExact,
                (Scope::Exact, false) => MatchPriority::AliasExact,
                (Scope::Prefix, true) => MatchPriority::NamePrefix,
                (Scope::Prefix, false) => MatchPriority::AliasPrefix,
                (Scope::Partial, true) => MatchPriority::NamePartial,
                (Scope::Partial, false) => MatchPriority::AliasPartial,
            };

            best.entry(Arc::clone(&p.doc))
                .and_modify(|current| {
                    if prio < *current {
                        *current = prio;
                    }
                })
                .or_insert(prio);
        }
    }

    /// 前方一致の収集 (FST range)
    fn collect_prefix(&self, prefix: &str, best: &mut HashMap<Arc<String>, MatchPriority>) {
        if prefix.is_empty() {
            return;
        }
        let Some(map) = &self.cache.fst_map else { return; };

        // prefix <= key < prefix+MAX という範囲でストリームを作成
        let upper = format!("{}{}", prefix, char::MAX);
        let mut stream = map.range().ge(prefix).lt(&upper).into_stream();

        while let Some((_, idx)) = stream.next() {
            if let Some(postings) = self.cache.postings.get(idx as usize) {
                self.record_hits(postings, Scope::Prefix, best);
            }
        }
    }

    /// レーベンシュタイン距離 <=1 の近傍収集
    fn collect_fuzzy(&self, key: &str, best: &mut HashMap<Arc<String>, MatchPriority>) {
        // 1文字（コードポイント）クエリは誤爆が多いのでスキップ
        if key.chars().count() <= 1 {
            return;
        }
        let Some(map) = &self.cache.fst_map else { return; };

        if let Ok(lev) = fst::automaton::Levenshtein::new(key, 1) {
            let mut stream = map.search(lev).into_stream();
            while let Some((_, idx)) = stream.next() {
                if let Some(postings) = self.cache.postings.get(idx as usize) {
                    self.record_hits(postings, Scope::Partial, best);
                }
            }
        }
    }

    /// 部分一致（文字列 contains）の収集。小規模なので線形スキャンで十分。
    fn collect_partial(
        &mut self,
        variants: &[String],
        best: &mut HashMap<Arc<String>, MatchPriority>,
    ) {
        for (doc_name, aliases) in self.doc_aliases.iter() {
            let doc_lower = self.cache.get_lowercase(doc_name);
            let doc_hiragana = self.cache.get_hiragana(doc_name);

            // 名前の部分一致
            let mut name_matched = false;
            for q in variants {
                if doc_lower.contains(q)
                    || doc_hiragana
                        .as_ref()
                        .map_or(false, |h| h.contains(q))
                {
                    self.record_hits(
                        &[Posting {
                            doc: Arc::clone(doc_name),
                            is_name: true,
                        }],
                        Scope::Partial,
                        best,
                    );
                    name_matched = true;
                    break;
                }
            }

            // エイリアスの部分一致
            if name_matched {
                continue;
            }
            for alias in aliases {
                let alias_lower = self.cache.get_lowercase(alias);
                let alias_hiragana = self.cache.get_hiragana(alias);

                for q in variants {
                    if alias_lower.contains(q)
                        || alias_hiragana
                            .as_ref()
                            .map_or(false, |h| h.contains(q))
                    {
                        self.record_hits(
                            &[Posting {
                                doc: Arc::clone(doc_name),
                                is_name: false,
                            }],
                            Scope::Partial,
                            best,
                        );
                        break;
                    }
                }
            }
        }
    }
}
