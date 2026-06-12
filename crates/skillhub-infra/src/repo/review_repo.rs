//! TODO: implement repository against PgPool.

use crate::db::PgPool;

pub struct PgReviewRepo {
    pub pool: PgPool,
}
