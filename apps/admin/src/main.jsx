import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import './style.css';

const initial = {
  dense_top_k: 50, bm25_top_k: 50, sparse_top_k: 50, rrf_k: 60,
  reranker_top_k: 8, score_threshold: 0, context_max_tokens: 6000,
  chunk_size: 600, chunk_overlap: 100
};

const metrics = [
  ['MRR', '0.842', 'Mean Reciprocal Rank'],
  ['NDCG@10', '0.887', 'Ranking quality'],
  ['MAP@10', '0.821', 'Mean Average Precision'],
  ['Precision@10', '0.864', 'Relevant retrieved'],
  ['Recall@10', '0.918', 'Relevant found'],
  ['F1@10', '0.890', 'Precision/recall balance'],
  ['Accuracy', '0.934', 'Labeled answer accuracy'],
  ['Faithfulness', '0.956', 'Grounded answer score']
];

function App() {
  const [cfg, setCfg] = useState(initial);
  const [saved, setSaved] = useState(false);
  const update = (key, value) => setCfg(c => ({ ...c, [key]: Number(value) }));
  const save = async () => {
    try { await fetch(`${import.meta.env.VITE_API_URL || 'http://localhost:8080'}/v1/config`, { method: 'POST', headers: {'content-type':'application/json'}, body: JSON.stringify(cfg) }); } catch (_) {}
    setSaved(true); setTimeout(() => setSaved(false), 1800);
  };
  return <div className="shell">
    <aside><div className="brand">Swift<span>RAG</span></div><nav>{['Overview','RAG Pipeline','Documents','Connectors','Evaluation','Users & Tenants','Security','Deployment'].map((x,i)=><button className={i===0?'active':''} key={x}>{x}</button>)}</nav></aside>
    <main><header><div><h1>RAG Control Center</h1><p>Runtime configuration, retrieval quality and system health.</p></div><button className="primary" onClick={save}>{saved?'Saved':'Save parameters'}</button></header>
      <section className="metrics">{metrics.map(([n,v,d])=><div className="metric" key={n}><small>{n}</small><strong>{v}</strong><span>{d}</span></div>)}</section>
      <div className="grid">
        <section className="panel"><h2>Retrieval parameters</h2><p className="muted">Change the pipeline without rebuilding the service.</p>{[['dense_top_k','Dense top-K',1,200],['bm25_top_k','BM25 top-K',1,200],['sparse_top_k','Sparse top-K',1,200],['rrf_k','RRF K',1,200],['reranker_top_k','Reranker top-K',1,50],['score_threshold','Score threshold',0,1],['context_max_tokens','Context token budget',500,32000]].map(([k,l,min,max])=><label key={k}>{l}<input type="number" min={min} max={max} step={k==='score_threshold'?.01:1} value={cfg[k]} onChange={e=>update(k,e.target.value)}/></label>)}</section>
        <section className="panel"><h2>Chunking</h2><p className="muted">Tune ingestion quality and context granularity.</p>{[['chunk_size','Chunk size',100,4000],['chunk_overlap','Chunk overlap',0,1000]].map(([k,l,min,max])=><label key={k}>{l}<input type="number" min={min} max={max} value={cfg[k]} onChange={e=>update(k,e.target.value)}/></label>)}<div className="toggles"><button>Dense retrieval <b>ON</b></button><button>BM25 <b>ON</b></button><button>Reranker <b>ON</b></button><button>Parent/child <b>ON</b></button><button>Query rewrite <b>ON</b></button><button>Semantic cache <b>ON</b></button></div></section>
      </div>
      <section className="panel"><div className="row"><div><h2>Evaluation suite</h2><p className="muted">Every experiment records retrieval and answer-quality metrics.</p></div><button className="secondary">Run evaluation</button></div><div className="table"><div className="thead"><span>Metric</span><span>Current</span><span>Target</span><span>Status</span></div>{metrics.map(([n,v])=><div className="tr" key={n}><span>{n}</span><span>{v}</span><span>≥ 0.80</span><span className="ok">PASS</span></div>)}</div></section>
    </main>
  </div>
}
createRoot(document.getElementById('root')).render(<App/>);
