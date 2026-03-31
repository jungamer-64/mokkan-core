use core::{future::Future, pin::Pin};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub fn boxed<'a, F, T>(future: F) -> BoxFuture<'a, T>
where
    F: Future<Output = T> + Send + 'a,
{
    Box::pin(future)
}
