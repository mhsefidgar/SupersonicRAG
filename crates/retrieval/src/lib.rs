pub fn rrf_score(ranks: &[usize], k: usize) -> f64 {
    ranks.iter().map(|r| 1.0 / (k as f64 + *r as f64)).sum()
}
