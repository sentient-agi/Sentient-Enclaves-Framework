use async_std::fs::File;
use async_std::io;
use async_std::prelude::*;
use async_std::task::{Context, Poll};
use futures::task::noop_waker;
use sha2::{Digest, Sha512};
use std::pin::Pin;

#[async_std::main]
async fn main() -> io::Result<()> {
    // Open the file for reading
    let mut file = File::open("example.txt").await?;

    // Create a hasher
    let mut hasher = Sha512::new();

    // Create a writer adapter for the hasher
    let mut hasher_writer = io::BufWriter::new(HasherWriter::new(&mut hasher));

    // Create the future for the copy operation
    let mut copy_future = Box::pin(io::copy(&mut file, &mut hasher_writer));

    // Poll the future to check readiness
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    // Check readiness of the `Future`
    match copy_future.as_mut().poll(&mut cx) {
        Poll::Ready(result) => {
            result?; // Ensure the copy completed successfully
            println!("Copy operation completed immediately.");
        }
        Poll::Pending => {
            println!("Copy operation not ready; awaiting completion.");
            // Await the future if it is not ready
            copy_future.await?;
        }
    }

    // Finalize the hash
    let hash_result = hasher.finalize();
    println!("SHA-512 Hash: {:x}", hash_result);

    Ok(())
}

/// A wrapper around a hasher to implement `io::Write`.
struct HasherWriter<'a, H: Digest> {
    hasher: &'a mut H,
}

impl<'a, H: Digest> HasherWriter<'a, H> {
    fn new(hasher: &'a mut H) -> Self {
        Self { hasher }
    }
}

impl<H: Digest> io::Write for HasherWriter<'_, H> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.hasher.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
