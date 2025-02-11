use std::pin::Pin;
use std::task::{Context, Poll};
use std::future::Future;

/// A function to check if a Future is ready.
///
/// # Arguments
/// * `future` - A pinned reference to the Future.
/// * `cx` - A mutable reference to the Context.
///
/// # Returns
/// * `true` if the Future is ready, otherwise `false`.
fn is_future_ready<F>(future: &mut Pin<&mut F>, cx: &mut Context<'_>) -> bool
where
    F: Future,
{
    matches!(future.poll(cx), Poll::Ready(_))
}

#[tokio::main]
async fn main() {
    use tokio::io::{self, AsyncRead, AsyncWrite};

    let (mut reader, mut writer) = io::duplex(64);

    // Example future for a copy operation
    let copy_future = tokio::io::copy(&mut reader, &mut writer);
    let mut pinned_future = Box::pin(copy_future);

    // Example context (using `noop_waker` for simplicity)
    let waker = futures::task::noop_waker();
    let mut cx = Context::from_waker(&waker);

    if is_future_ready(&mut pinned_future, &mut cx) {
        println!("Future is ready for I/O copy operation!");
    } else {
        println!("Future is not ready yet.");
    }
}
