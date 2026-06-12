//! TODO: implement repository against PgPool.

use crate::db::PgPool;

pub struct PgTokenRepo {
    pub pool: PgPool,
}
