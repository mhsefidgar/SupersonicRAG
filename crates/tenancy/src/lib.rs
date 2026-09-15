#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantContext {
    pub organization_id: String,
    pub project_id: String,
}
