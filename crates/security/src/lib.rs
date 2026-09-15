#[derive(Debug, Clone, Copy)]
pub struct SecurityPolicy {
    pub enforce_tenant_filter: bool,
    pub enforce_acl: bool,
    pub prompt_injection_guard: bool,
    pub audit_logging: bool,
}
