#[derive(Debug, Clone)]
pub struct DocumentRef {
    pub tenant_id: String,
    pub document_id: String,
    pub content_hash: String,
}
