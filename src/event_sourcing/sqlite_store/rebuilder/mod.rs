mod pg_rebuilder;

use async_trait::async_trait;

use esrs::Aggregate;

#[async_trait]
pub trait Rebuilder<A>
where
    A: Aggregate,
{
    type Executor;
    type Error: std::error::Error;

    async fn all_at_once(&self, executor: Self::Executor) -> Result<(), Self::Error>;
}
