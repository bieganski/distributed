/// When doing IO, there usually is not much work to be done by CPU. If your
/// software needs to be very efficient, you might consider the effort of
/// modelling it as asynchronous code.
/// Sadly, futures are not the most mature feature of Rust. This code needs to
/// run on tokio runtime (one of a few libraries - others are futures and async-std),
/// because we use reqwest library (for the sake of brevity). So the base library which
/// provides futures can lock you from using some other libraries, which should not be the case
/// if you think about it. But this is how it is/was in 2020.
async fn fetch_multiple_at_once() -> Result<(), Box<dyn std::error::Error>> {
    // join_all transforms vector of futures into a future
    let search_engines_replies = futures::future::join_all(
        vec![
            "https://google.com",
            "https://duckduckgo.com",
            "https://www.bing.com",
        ]
        .into_iter()
        .map(reqwest::get),
    )
    // Await takes as much time to come back as the longest download - HTTP requests
    // are done at the same time under the hood, all hidden by Future trait and tokio runtime.
    .await
    .into_iter()
    .flatten()
    .collect::<Vec<reqwest::Response>>();

    println!("{:#?}", search_engines_replies);
    Ok(())
}

fn main() {
    let mut rt = tokio::runtime::Runtime::new().unwrap();
    // Next line needs internet connection.
    rt.block_on(fetch_multiple_at_once()).unwrap();
}
