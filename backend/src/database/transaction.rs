use sqlx::{Sqlite, Transaction};

pub type DatabaseTransaction<'a> = Transaction<'a, Sqlite>;
