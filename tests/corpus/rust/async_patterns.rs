//! Test Corpus: Async/Await Patterns
//!
//! Expected symbols: 10+ async functions
//!
//! Real-world patterns tested:
//! - async fn
//! - .await usage
//! - async blocks
//! - Future trait usage

use std::future::Future;

pub async fn simple_async() -> i32 {
    42
}

async fn async_with_await() -> String {
    let result = simple_async().await;
    result.to_string()
}

pub async fn async_result() -> Result<String, String> {
    Ok("success".to_string())
}

async fn multiple_awaits() -> Result<i32, String> {
    let a = simple_async().await;
    let b = simple_async().await;
    Ok(a + b)
}

pub async fn async_generic<T: Send>(value: T) -> T {
    value
}

fn returns_async_block() -> impl Future<Output = i32> {
    async { 42 }
}

pub async fn async_with_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    let val = async_result().await?;
    println!("{}", val);
    Ok(())
}

async fn async_closure_example() {
    let f = async || {
        42
    };
    let result = f().await;
}

pub async fn spawn_task() {
    tokio::spawn(async {
        println!("Spawned task");
    });
}

async fn select_example() {
    tokio::select! {
        _ = simple_async() => println!("First"),
        _ = async_result() => println!("Second"),
    }
}
