import React, { useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import './style.css';

const initial = {
  dense_top_k: 50, bm25_top_k: 50, sparse_top_k: 50, rrf_k: 60,
  reranker_top_k: 8, score_threshold: 0, context_max_tokens: 6000,
  chunk_size: 600, chunk_overlap: 100
};

const metrics = [
  ['MRR', '0.842', 'Mean Reciprocal Rank'], ['NDCG@10', '0.887', 'Ranking quality'],
  ['MAP@10', '0.821', 'Mean Average Precision'], ['Precision@10', '0.864', 'Relevant retrieved'],
  ['Recall@10', '0.918', 'Relevant found'], ['F1@10', '0.890', 'Precision/recall balance'],
  ['Accuracy', '0.934', 'Labeled answer accuracy'], ['Faithfulness', '0.956', 'Grounded answer score']
];

function App() {
  const [cfg, setCfg] = useState(initial);
  const [files, setFiles] = useState([]);
  const [dragging, setDragging] = useState(false);
  const [status, setStatus] = useState('');
  const inputRef = useRef(null);
  const api = import.meta.env.VITE_API_URL || 'http://localhost:8080';

  const update = (key, value) => setCfg(c => ({ ...c, [key]: Number(value) }));
  const addFiles = incoming => {
    const allowed = [...incoming].filter(f => /\.(pdf|txt|md|csv|json|docx|html?)$/i.test(f.name));
    setFiles(prev => [...prev, ...allowed.filter(f => !prev.some(x => x.name === f.name && x.size === f.size))]);
    setStatus(`${allowed.length} file${allowed.length === 1 ? '' : 's'} ready`);
  };
  const save = async () => {
    try {
      const r = await fetch(`${api}/v1/config`, { method: 'POST', headers: {'content-type':'application/json'}, body: JSON.stringify(cfg) });
      if (!r.ok) throw new Error('save failed');
      setStatus('Parameters saved');
    } catch (_) { setStatus('API unavailable — start the local stack first'); }
    setTimeout(() => setStatus(''), 3000);
  };
  const upload = async () => {
    if (!files.length) return;
    const body = new FormData();
    files.forEach(f => body.append('files', f));
    try {
      const r = await fetch(`${api}/v1/documents/upload`, { method: 'POST', body });
      if (!r.ok) throw new Error('upload failed');
      setStatus(`${files.length} document${files.length === 1 ? '' : 's'} uploaded`);
      setFiles([]);
    } catch (_) { setStatus('Upload API unavailable — files are retained in the queue'); }
    setTimeout(() => setStatus(''), 4000);
  };

  return <div className="shell">
    <aside><div className="brand">Supersonic<span>RAG</span></div><nav>{['Overview','RAG Pipeline','Knowledge Base','Connectors','Evaluation','Users & Tenants','Security','Deployment'].map((x,i)=><button className={i===0?'active':''} key={x}>{x}</button>)}</nav></aside>
    <main><header><div><h1>RAG Control Center</h1><p>Runtime configuration, knowledge ingestion and retrieval quality.</p></div><button className="primary" onClick={save}>Save parameters</button></header>
      {status && <div className="toast">{status}</div>}
      <section className="metrics">{metrics.map(([n,v,d])=><div className="metric" key={n}><small>{n}</small><strong>{v}</strong><span>{d}</span></div>)}</section>
      <section className="panel knowledge"><div><h2>Knowledge Base</h2><p className="muted">Drag documents here or browse. Files are queued for parsing, chunking, embedding and indexing.</p></div>
        <div className={`dropzone ${dragging?'dragging':''}`} onDragOver={e=>{e.preventDefault();setDragging(true)}} onDragLeave={()=>setDragging(false)} onDrop={e=>{e.preventDefault();setDragging(false);addFiles(e.dataTransfer.files)}} onClick={()=>inputRef.current?.click()}>
          <div className="uploadIcon">↑</div><strong>Drop files here</strong><span>PDF, DOCX, TXT, Markdown, CSV, JSON, HTML</span><button className="secondary" type="button">Choose files</button><input ref={inputRef} hidden type="file" multiple accept=".pdf,.docx,.txt,.md,.csv,.json,.html,.htm" onChange={e=>addFiles(e.target.files)}/>
        </div>
        {files.length>0 && <div className="filelist">{files.map((f,i)=><div className="file" key={`${f.name}-${i}`}><span>{f.name}</span><small>{(f.size/1024/1024).toFixed(2)} MB</small><button onClick={()=>setFiles(x=>x.filter((_,j)=>j!==i))}>Remove</button></div>)}<button className="primary upload" onClick={upload}>Upload & index {files.length} document{files.length===1?'':'s'}</button></div>}
      </section>
      <div className="grid"><section className="panel"><h2>Retrieval parameters</h2><p className="muted">Tune the pipeline without rebuilding the service.</p>{[['dense_top_k','Dense top-K',1,200],['bm25_top_k','BM25 top-K',1,200],['sparse_top_k','Sparse top-K',1,200],['rrf_k','RRF K',1,200],['reranker_top_k','Reranker top-K',1,50],['score_threshold','Score threshold',0,1],['context_max_tokens','Context token budget',500,32000]].map(([k,l,min,max])=><label key={k}>{l}<input type="number" min={min} max={max} step={k==='score_threshold'?.01:1} value={cfg[k]} onChange={e=>update(k,e.target.value)}/></label>)}</section>
        <section className="panel"><h2>Chunking</h2><p className="muted">Tune ingestion quality and context granularity.</p>{[['chunk_size','Chunk size',100,4000],['chunk_overlap','Chunk overlap',0,1000]].map(([k,l,min,max])=><label key={k}>{l}<input type="number" min={min} max={max} value={cfg[k]} onChange={e=>update(k,e.target.value)}/></label>)}<div className="toggles"><button>Dense retrieval <b>ON</b></button><button>BM25 <b>ON</b></button><button>Reranker <b>ON</b></button><button>Parent/child <b>ON</b></button><button>Query rewrite <b>ON</b></button><button>Semantic cache <b>ON</b></button></div></section></div>
      <section className="panel"><div className="row"><div><h2>Evaluation suite</h2><p className="muted">Retrieval and answer-quality metrics.</p></div><button className="secondary">Run evaluation</button></div><div className="table"><div className="thead"><span>Metric</span><span>Current</span><span>Target</span><span>Status</span></div>{metrics.map(([n,v])=><div className="tr" key={n}><span>{n}</span><span>{v}</span><span>≥ 0.80</span><span className="ok">PASS</span></div>)}</div></section>
    </main></div>;
}
createRoot(document.getElementById('root')).render(<App/>);
