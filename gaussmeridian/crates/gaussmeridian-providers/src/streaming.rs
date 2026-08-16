//! Streaming utilities for providers

use futures::{Stream, StreamExt};
use std::time::Duration;
use tokio::time::interval;

/// Transform a stream with a mapping function
pub fn transform_stream<S, F, T>(stream: S, transform: F) -> impl Stream<Item = T>
where
    S: Stream + Unpin,
    F: Fn(S::Item) -> T + Send + Sync + 'static,
{
    stream.map(transform)
}

/// Filter a stream with a predicate
pub fn filter_stream<S, F>(stream: S, predicate: F) -> impl Stream<Item = S::Item>
where
    S: Stream + Unpin,
    F: Fn(&S::Item) -> bool + Send + Sync + 'static,
{
    stream.filter(move |item| std::future::ready(predicate(item)))
}

/// Buffer stream items into batches
pub fn buffer_stream<S>(stream: S, buffer_size: usize) -> impl Stream<Item = Vec<S::Item>>
where
    S: Stream + Unpin,
    S::Item: Clone,
{
    stream.ready_chunks(buffer_size)
}

/// Create a stream that yields items at regular intervals
pub fn interval_stream<T, F>(interval_duration: Duration, generator: F) -> impl Stream<Item = T>
where
    F: Fn() -> T + Send + Sync + Clone + 'static,
    T: Clone,
{
    let interval = interval(interval_duration);
    let generator = generator.clone();
    futures::stream::unfold(
        (interval, generator),
        |(mut interval, generator)| async move {
            interval.tick().await;
            Some((generator(), (interval, generator)))
        },
    )
}

/// Merge multiple streams into one
pub fn merge_streams<S>(streams: Vec<S>) -> impl Stream<Item = S::Item>
where
    S: Stream + Unpin,
{
    futures::stream::select_all(streams)
}

/// Create a stream that yields items from an iterator
pub fn from_iter<I>(iter: I) -> impl Stream<Item = I::Item>
where
    I: IntoIterator,
    I::Item: Clone,
{
    futures::stream::iter(iter)
}

/// Create a stream that yields a single item
pub fn once<T>(item: T) -> impl Stream<Item = T>
where
    T: Clone,
{
    futures::stream::once(async move { item })
}

/// Create an empty stream
pub fn empty<T>() -> impl Stream<Item = T> {
    futures::stream::empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_transform_stream() {
        let stream = futures::stream::iter(vec![1, 2, 3]);
        let transformed = transform_stream(stream, |x| x * 2);

        let results: Vec<_> = transformed.collect().await;
        assert_eq!(results, vec![2, 4, 6]);
    }

    #[tokio::test]
    async fn test_filter_stream() {
        let stream = futures::stream::iter(vec![1, 2, 3, 4, 5]);
        let filtered = filter_stream(stream, |&x| x % 2 == 0);

        let results: Vec<_> = filtered.collect().await;
        assert_eq!(results, vec![2, 4]);
    }

    #[tokio::test]
    async fn test_buffer_stream() {
        let stream = futures::stream::iter(vec![1, 2, 3, 4, 5]);
        let buffered = buffer_stream(stream, 2);

        let results: Vec<_> = buffered.collect().await;
        assert_eq!(results, vec![vec![1, 2], vec![3, 4], vec![5]]);
    }
}
