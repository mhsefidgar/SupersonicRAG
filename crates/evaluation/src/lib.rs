use std::collections::HashSet;

/// Binary relevance ranking metrics used by the SwiftRAG evaluation engine.
pub fn reciprocal_rank(relevances: &[bool]) -> f64 {
    relevances.iter().position(|&r| r).map(|i| 1.0 / (i as f64 + 1.0)).unwrap_or(0.0)
}

pub fn precision_at_k(relevances: &[bool], k: usize) -> f64 {
    if k == 0 { return 0.0; }
    let n = relevances.iter().take(k).filter(|&&r| r).count();
    n as f64 / k.min(relevances.len()).max(1) as f64
}

pub fn recall_at_k(relevances: &[bool], total_relevant: usize, k: usize) -> f64 {
    if total_relevant == 0 { return 0.0; }
    relevances.iter().take(k).filter(|&&r| r).count() as f64 / total_relevant as f64
}

pub fn f1(precision: f64, recall: f64) -> f64 {
    if precision + recall == 0.0 { 0.0 } else { 2.0 * precision * recall / (precision + recall) }
}

pub fn average_precision(relevances: &[bool], total_relevant: usize) -> f64 {
    if total_relevant == 0 { return 0.0; }
    let mut hits = 0usize;
    let mut sum = 0.0;
    for (i, &relevant) in relevances.iter().enumerate() {
        if relevant {
            hits += 1;
            sum += hits as f64 / (i + 1) as f64;
        }
    }
    sum / total_relevant as f64
}

pub fn dcg(relevances: &[f64], k: usize) -> f64 {
    relevances.iter().take(k).enumerate().map(|(i, rel)| {
        let rank = i as f64 + 2.0;
        (2.0_f64.powf(*rel) - 1.0) / rank.log2()
    }).sum()
}

pub fn ndcg(relevances: &[f64], k: usize) -> f64 {
    let actual = dcg(relevances, k);
    let mut ideal = relevances.to_vec();
    ideal.sort_by(|a,b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let best = dcg(&ideal, k);
    if best == 0.0 { 0.0 } else { actual / best }
}

pub fn accuracy(predicted: &[bool], expected: &[bool]) -> f64 {
    if predicted.is_empty() || predicted.len() != expected.len() { return 0.0; }
    predicted.iter().zip(expected).filter(|(a,b)| a == b).count() as f64 / predicted.len() as f64
}

pub fn macro_mean(values: &[f64]) -> f64 {
    if values.is_empty() { 0.0 } else { values.iter().sum::<f64>() / values.len() as f64 }
}

pub fn unique_relevant(ids: &[String]) -> usize {
    ids.iter().collect::<HashSet<_>>().len()
}
