//! TODO: implement repository against PgPool.

use crate::db::PgPool;

pub struct PgNamespaceRepo {
    pub pool: PgPool,
}
