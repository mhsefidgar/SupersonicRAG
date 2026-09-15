pub mod pipeline {
    #[derive(Debug, Clone, Copy)]
    pub struct RetrievalConfig {
        pub dense_top_k: usize,
        pub bm25_top_k: usize,
        pub rrf_k: usize,
        pub reranker_top_k: usize,
    }
}
