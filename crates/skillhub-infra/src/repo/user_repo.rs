//! TODO: implement repository against PgPool.

use crate::db::PgPool;

pub struct PgUserRepo {
    pub pool: PgPool,
}
