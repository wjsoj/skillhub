//! TODO: implement repository against PgPool.

use crate::db::PgPool;

pub struct PgAuditRepo {
    pub pool: PgPool,
}
