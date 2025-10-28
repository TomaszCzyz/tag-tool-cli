use esrs::Aggregate;
use sqlx::{Database, Sqlite};

pub trait StatementsHandler<D>
where
    D: Database,
{
    fn new<A>() -> Self
    where
        A: Aggregate;
    fn by_aggregate_id(&self) -> &str;
    fn select_all(&self) -> &str;
    fn insert(&self) -> &str;
    fn delete_by_aggregate_id(&self) -> &str;
}

#[derive(Clone, Debug)]
pub struct Statements {
    select_by_aggregate_id: String,
    select_all: String,
    insert: String,
    delete_by_aggregate_id: String,
}

impl StatementsHandler<Sqlite> for Statements {
    fn new<A>() -> Self
    where
        A: Aggregate,
    {
        let table_name: String = format!("{}_events", A::NAME);

        Self {
            select_by_aggregate_id: format!(include_str!("statements/select_by_aggregate_id.sql"), table_name),
            select_all: format!(include_str!("statements/select_all.sql"), table_name),
            insert: format!(include_str!("statements/insert.sql"), table_name),
            delete_by_aggregate_id: format!(include_str!("statements/delete_by_aggregate_id.sql"), table_name),
        }
    }

    fn by_aggregate_id(&self) -> &str {
        &self.select_by_aggregate_id
    }

    fn select_all(&self) -> &str {
        &self.select_all
    }

    fn insert(&self) -> &str {
        &self.insert
    }

    fn delete_by_aggregate_id(&self) -> &str {
        &self.delete_by_aggregate_id
    }
}
