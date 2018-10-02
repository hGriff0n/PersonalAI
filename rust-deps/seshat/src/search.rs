
use array_tool::vec::*;

use super::index as idx;

pub fn default_search(query: &str, index: &idx::Index) -> Vec<String> {
    search(query, index, &intersect_rank)
}

pub type RankingFunction = Fn(Vec<idx::ElementList>) -> idx::ElementList;
pub fn search(query: &str, index: &idx::Index, page_rank: &RankingFunction) -> idx::ElementList {
    let results = index.retrieve(query);
    page_rank(results)
}

pub fn intersect_rank(results: Vec<idx::ElementList>) -> idx::ElementList {
    let mut iter = results.into_iter();
    iter.next()
        .map(|first| iter.fold(first,
            |res, word_res| res.intersect(word_res)))
        .unwrap_or(Vec::new())
}
