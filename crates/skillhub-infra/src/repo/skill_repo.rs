//! TODO: implement repository against PgPool.

use crate::db::PgPool;

pub struct PgSkillRepo {
    pub pool: PgPool,
}
