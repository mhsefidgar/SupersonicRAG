use std::collections::HashSet;

#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct Scorecard { pub mrr:f64, pub ndcg:f64, pub map:f64, pub precision:f64, pub recall:f64, pub f1:f64, pub accuracy:f64 }

pub fn reciprocal_rank(r:&[bool])->f64{r.iter().position(|&x|x).map(|i|1.0/(i as f64+1.0)).unwrap_or(0.0)}
pub fn precision_at_k(r:&[bool],k:usize)->f64{let n=k.min(r.len());if n==0{0.0}else{r[..n].iter().filter(|&&x|x).count() as f64/n as f64}}
pub fn recall_at_k(r:&[bool],total:usize,k:usize)->f64{if total==0{0.0}else{r.iter().take(k).filter(|&&x|x).count() as f64/total as f64}}
pub fn f1(p:f64,r:f64)->f64{if p+r==0.0{0.0}else{2.0*p*r/(p+r)}}
pub fn average_precision(r:&[bool],total:usize)->f64{if total==0{return 0.0}let(mut hits,mut sum)=(0usize,0.0);for(i,&x)in r.iter().enumerate(){if x{hits+=1;sum+=hits as f64/(i+1)as f64}}sum/total as f64}
pub fn dcg(r:&[f64],k:usize)->f64{r.iter().take(k).enumerate().map(|(i,x)|((2.0_f64.powf(*x)-1.0)/(i as f64+2.0).log2())).sum()}
pub fn ndcg(r:&[f64],k:usize)->f64{let a=dcg(r,k);let mut ideal=r.to_vec();ideal.sort_by(|a,b|b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));let b=dcg(&ideal,k);if b==0.0{0.0}else{a/b}}
pub fn accuracy(a:&[bool],b:&[bool])->f64{if a.is_empty()||a.len()!=b.len(){0.0}else{a.iter().zip(b).filter(|(x,y)|x==y).count()as f64/a.len()as f64}}
pub fn macro_mean(v:&[f64])->f64{if v.is_empty(){0.0}else{v.iter().sum::<f64>()/v.len()as f64}}
pub fn unique_relevant(ids:&[String])->usize{ids.iter().collect::<HashSet<_>>().len()}

#[cfg(test)]
mod tests{use super::*;#[test]fn ranking_metrics_are_bounded(){let r=[true,false,true,false];let p=precision_at_k(&r,4);let rec=recall_at_k(&r,2,4);assert!((0.0..=1.0).contains(&reciprocal_rank(&r)));assert!((0.0..=1.0).contains(&p));assert!((0.0..=1.0).contains(&rec));assert!((0.0..=1.0).contains(&f1(p,rec)));assert!((0.0..=1.0).contains(&average_precision(&r,2)));assert!((0.0..=1.0).contains(&ndcg(&[3.0,2.0,0.0],3)));}#[test]fn accuracy_exact(){assert_eq!(accuracy(&[true,false,true],&[true,true,true]),2.0/3.0);}}
