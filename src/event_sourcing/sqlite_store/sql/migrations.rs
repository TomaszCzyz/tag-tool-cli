use crate::statement;
use async_trait::async_trait;
use esrs::Aggregate;
use sqlx::sqlite::SqliteQueryResult;
use sqlx::{Database, Error, Pool, Sqlite, Transaction};

/// Trait used to handle current code migrations.
#[async_trait]
pub trait MigrationsHandler<D>
where
    D: Database,
{
    async fn run<A>(pool: &Pool<D>) -> Result<(), Error>
    where
        A: Aggregate;
}

pub struct Migrations;

#[async_trait]
impl MigrationsHandler<Sqlite> for Migrations {
    async fn run<A>(pool: &Pool<Sqlite>) -> Result<(), Error>
    where
        A: Aggregate,
    {
        let mut transaction: Transaction<Sqlite> = pool.begin().await?;

        let migrations: Vec<String> = vec![
            statement!("migrations/01_create_table.sql", A),
            statement!("migrations/02_create_index.sql", A),
            statement!("migrations/03_create_unique_constraint.sql", A),
        ];

        for migration in migrations {
            let _: SqliteQueryResult = sqlx::query(migration.as_str()).execute(&mut *transaction).await?;
        }

        transaction.commit().await
    }
}

#[cfg(test)]
mod tests {
    use crate::event_sourcing::sqlite_store::sql::migrations::{Migrations, MigrationsHandler};
    use esrs::Aggregate;
    use sqlx::{Pool, Sqlite};

    #[sqlx::test]
    async fn can_read_sqlite_migrations(pool: Pool<Sqlite>) {
        let result = Migrations::run::<TestAggregate>(&pool).await;
        dbg!(&result);
        assert!(result.is_ok());
    }

    #[derive(Debug, thiserror::Error)]
    pub enum Error {}

    pub struct TestAggregate;

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct TestEvent;

    impl Aggregate for TestAggregate {
        const NAME: &'static str = "test";
        type State = ();
        type Command = ();
        type Event = TestEvent;
        type Error = Error;

        fn handle_command(_state: &Self::State, _command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
            Ok(vec![])
        }

        fn apply_event(_state: Self::State, _payload: Self::Event) -> Self::State {}
    }
}
