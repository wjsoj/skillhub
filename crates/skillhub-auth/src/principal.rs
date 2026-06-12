use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Anonymous,
    User,
    NamespaceMember,
    NamespaceAdmin,
    NamespaceOwner,
    SuperAdmin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principal {
    pub user_id: Option<Uuid>,
    pub username: Option<String>,
    pub role: Role,
    pub scopes: Vec<String>,
}

impl Principal {
    pub fn anonymous() -> Self {
        Self {
            user_id: None,
            username: None,
            role: Role::Anonymous,
            scopes: Vec::new(),
        }
    }
}
