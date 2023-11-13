//! # Tokio Notify Aggregator
//!
//! `tokio_notify_aggregator` provides a mechanism to aggregate multiple Tokio
//! [`Notify`] instances into a single notification source. This allows for
//! waiting on notifications from any of a set of [`Notify`] instances.
//!
//! ## Usage
//!
//! [`NotifyAggregator`] can be used in scenarios where you need to wait for notifications from
//! multiple sources, but are not concerned with which specific source the notification came from.
//!
//! To wait for notifications, [`NotifyAggregator`] mimics the API of [`Notify`] - that is a call
//! to [`NotifyAggregator::notified`] can be awaited in the same manner as [`Notify::notified`].
//!
//! While a [`NotifyAggregator`] can be created and awaited on in a single statement, where
//! appropriate it is recommended to create the aggregator and then re-await it as needed. This
//! prevents (re)spawning of new tasks to handle each registered [`Notify`] instance.
//!
//! The aggregator also includes a graceful shutdown mechanism. When the `NotifyAggregator` is
//! dropped, it automatically signals all registered `Notify` tasks to stop, preventing potential
//! resource leaks due to never ending tasks.
//!
//! ### Examples
//!
//! Basic usage involves creating a [`NotifyAggregator`], adding [`Notify`] instances to it,
//! and then awaiting notifications:
//! ```no_run
//! use tokio_notify_aggregator::NotifyAggregator;
//! use std::sync::Arc;
//! use tokio::sync::Notify;
//!
//! #[tokio::main]
//! async fn main() {
//!     let aggregator = NotifyAggregator::new();
//!     let notifier = Arc::new(Notify::new());
//!     aggregator.add_notifier(notifier.clone());
//!
//!     // Wait for a notification
//!     aggregator.notified().await;
//! }
//! ```
//!
//! [`NotifyAggregator`] also supports initialisation from an iterator of [`Arc<Notify>`]
//! instances and chained additions of [`Arc<Notify>`].
//!
//! Creating a `NotifyAggregator` from an iterator:
//! ```no_run
//! use tokio_notify_aggregator::NotifyAggregator;
//! use std::sync::Arc;
//! use tokio::sync::Notify;
//!
//! #[tokio::main]
//! async fn main() {
//!    let notifiers = vec![Arc::new(Notify::new()), Arc::new(Notify::new())];
//!    let aggregator = NotifyAggregator::from_iter(notifiers.into_iter());
//!
//!    // Wait for a notification
//!    aggregator.notified().await;
//! }
//! ```
//!
//! Chained addition of [`Arc<Notify>`] instances. If only a single notification requires awaiting,
//! creation and waiting be done as a single statement:
//! ```no_run
//! use tokio_notify_aggregator::NotifyAggregator;
//! use std::sync::Arc;
//! use tokio::sync::Notify;
//!
//! #[tokio::main]
//! async fn main() {
//!     let notifier1 = Arc::new(Notify::new());
//!     let notifier2 = Arc::new(Notify::new());
//!
//!     // Create a temporary aggregator, add notifiers, and await a notification
//!     NotifyAggregator::new()
//!         .add_notifier(notifier1.clone())
//!         .add_notifier(notifier2.clone())
//!         .notified().await;
//! }
//! ```
//! [`Notify`]: https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html

// Include ReadMe tests
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;

use std::sync::Arc;
use tokio::sync::futures::Notified;
use tokio::sync::Notify;

/// This struct is used to aggregate multiple [`Notify`] instances into
/// effectively a single [`Notify`] instance.
///
/// Calling [`NotifyAggregator::notified`] will wait for a notification on any of the registered
/// [`Notify`] instances.
///
/// [`Notify`]: https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html
#[derive(Clone, Debug, Default)]
pub struct NotifyAggregator {
    // The internal [`Notify`] instance to be triggered by any of the registered [`Notify`]s
    composite_notifier: Arc<Notify>,
    // Used to signal all spawned tasks to stop when the aggregator is dropped
    cancel_notifier: Arc<Notify>,
}

impl NotifyAggregator {
    /// Constructs a new [`NotifyAggregator`].
    ///
    /// # Examples
    /// ```no_run
    /// use tokio_notify_aggregator::NotifyAggregator;
    ///
    /// let aggregator = NotifyAggregator::new();
    /// ```
    pub fn new() -> Self {
        NotifyAggregator {
            composite_notifier: Arc::new(Notify::new()),
            cancel_notifier: Arc::new(Notify::new()),
        }
    }

    /// Constructs a new [`NotifyAggregator`] with the [`Arc<Notify>`] instances from `vec`.
    ///
    /// # Examples
    /// ```no_run
    /// use std::sync::Arc;
    /// use tokio::sync::Notify;
    /// use tokio_notify_aggregator::NotifyAggregator;
    ///
    /// let notifiers = vec![Arc::new(Notify::new()), Arc::new(Notify::new())];
    /// let aggregator = NotifyAggregator::from_vec(&notifiers);
    /// ```
    pub fn from_vec(vec: &Vec<Arc<Notify>>) -> Self {
        Self::from(vec)
    }

    /// Registers a new [`Notify`] instance to be tracked.
    ///
    /// # Examples
    /// Simple usage:
    /// ```no_run
    /// use std::sync::Arc;
    /// use tokio::sync::Notify;
    /// use tokio_notify_aggregator::NotifyAggregator;
    ///
    /// let aggregator = NotifyAggregator::new();
    /// let notifier = Arc::new(Notify::new());
    /// aggregator.add_notifier(notifier.clone());
    /// ```
    ///
    /// Chained addition:
    /// ```no_run
    /// use std::sync::Arc;
    /// use tokio::sync::Notify;
    /// use tokio_notify_aggregator::NotifyAggregator;
    ///
    /// let notifier1 = Arc::new(Notify::new());
    /// let notifier2 = Arc::new(Notify::new());
    ///
    /// let aggregator = NotifyAggregator::new()
    ///     .add_notifier(notifier1.clone())
    ///     .add_notifier(notifier2.clone());
    /// ```
    ///
    /// [`Notify`]: https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html
    pub fn add_notifier(&self, notifier: Arc<Notify>) -> &Self {
        let internal_notifier = self.composite_notifier.clone();
        let cancel_notifier = self.cancel_notifier.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = notifier.notified() => internal_notifier.notify_waiters(),
                    _ = cancel_notifier.notified() => break,
                }
            }
        });
        self
    }

    /// Wait for a notification on any of the registered [`Notify`] instances.
    ///
    /// # Examples
    /// ```no_run
    /// use std::sync::Arc;
    /// use tokio::sync::Notify;
    ///
    /// use tokio_notify_aggregator::NotifyAggregator;
    ///
    /// #[tokio::main]
    /// async fn main() {
    /// let aggregator = NotifyAggregator::new();
    ///
    /// let notifier1 = Arc::new(Notify::new());
    /// let notifier2 = Arc::new(Notify::new());
    ///
    /// aggregator.add_notifier(notifier1.clone());
    /// aggregator.add_notifier(notifier2.clone());
    ///
    /// tokio::spawn(async move {
    ///     loop {
    ///         aggregator.notified().await;
    ///         println!("Notified!");
    ///     }
    /// });
    ///
    /// notifier1.notify_waiters();
    /// notifier2.notify_waiters();
    /// notifier1.notify_waiters();
    /// notifier2.notify_waiters();
    /// }
    /// ```
    ///
    /// [`Notify`]: https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html
    pub fn notified(&self) -> Notified<'_> {
        self.composite_notifier.notified()
    }
}

impl Drop for NotifyAggregator {
    /// Handles the cleanup logic for the [`NotifyAggregator`], by signaling all spawned tasks
    /// (one per registered [`Notify`]) to stop.
    ///
    /// [`Notify`]: https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html
    fn drop(&mut self) {
        self.cancel_notifier.notify_waiters();
    }
}

impl FromIterator<Arc<Notify>> for NotifyAggregator {
    /// Constructs a new [`NotifyAggregator`] with the [`Arc<Notify>`] instances from `iter`.
    ///
    /// # Examples
    /// ```no_run
    /// use std::sync::Arc;
    /// use tokio::sync::Notify;
    /// use tokio_notify_aggregator::NotifyAggregator;
    ///
    /// let notifiers = vec![Arc::new(Notify::new()), Arc::new(Notify::new())];
    /// let aggregator = NotifyAggregator::from_iter(notifiers.into_iter());
    /// ```
    fn from_iter<T: IntoIterator<Item=Arc<Notify>>>(iter: T) -> Self {
        let new_self = Self::new();
        for notifier in iter {
            new_self.add_notifier(notifier);
        }
        new_self
    }
}

impl From<Vec<Arc<Notify>>> for NotifyAggregator {
    /// Constructs a new [`NotifyAggregator`] with the [`Arc<Notify>`] instances from `vec`.
    ///
    /// # Examples
    /// ```no_run
    /// use std::sync::Arc;
    /// use tokio::sync::Notify;
    /// use tokio_notify_aggregator::NotifyAggregator;
    ///
    /// let notifiers = vec![Arc::new(Notify::new()), Arc::new(Notify::new())];
    /// let aggregator = NotifyAggregator::from(notifiers);
    /// ```
    fn from(vec: Vec<Arc<Notify>>) -> Self {
        Self::from_iter(vec.into_iter())
    }
}

impl<'a> From<&'a Vec<Arc<Notify>>> for NotifyAggregator {
    /// Constructs a new [`NotifyAggregator`] with the [`Arc<Notify>`] instances from `vec`.
    ///
    /// # Examples
    /// ```no_run
    /// use std::sync::Arc;
    /// use tokio::sync::Notify;
    /// use tokio_notify_aggregator::NotifyAggregator;
    ///
    /// let notifiers = vec![Arc::new(Notify::new()), Arc::new(Notify::new())];
    /// let aggregator = NotifyAggregator::from(&notifiers);
    /// ```
    fn from(vec: &'a Vec<Arc<Notify>>) -> Self {
        Self::from_iter(vec.iter().cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::Notify;
    use tokio::time::{self, Duration};

    async fn await_single_notification(aggregator: &NotifyAggregator, notify: Arc<Notify>) {
        tokio::spawn(async move {
            time::sleep(Duration::from_millis(100)).await;
            notify.notify_waiters();
        });

        tokio::time::timeout(Duration::from_millis(500), aggregator.notified()).await.unwrap();
    }

    #[tokio::test]
    async fn test_notified_single() {
        let aggregator = NotifyAggregator::new();
        let notifier = Arc::new(Notify::new());
        aggregator.add_notifier(notifier.clone());

        await_single_notification(&aggregator, notifier).await;
    }

    #[tokio::test]
    async fn test_notified_single_from_vec() {
        let notifier = Arc::new(Notify::new());
        let aggregator = NotifyAggregator::from_vec(&vec![notifier.clone()]);

        await_single_notification(&aggregator, notifier).await;
    }

    #[tokio::test]
    async fn test_notified_single_from_iter() {
        let notifier = Arc::new(Notify::new());
        let aggregator = NotifyAggregator::from_iter([notifier.clone()]);

        await_single_notification(&aggregator, notifier).await;
    }

    #[tokio::test]
    async fn test_notified_single_from_ref_vec() {
        let notifier = Arc::new(Notify::new());
        let aggregator = NotifyAggregator::from(&vec![notifier.clone()]);

        await_single_notification(&aggregator, notifier).await;
    }

    #[tokio::test]
    async fn test_notified_single_from_ref_vec_trait() {
        let notifier = Arc::new(Notify::new());
        let aggregator = NotifyAggregator::from(&vec![notifier.clone()]);

        await_single_notification(&aggregator, notifier).await;
    }

    #[tokio::test]
    async fn test_notified_single_from_vec_trait() {
        let notifier = Arc::new(Notify::new());
        let aggregator = NotifyAggregator::from(vec![notifier.clone()]);

        await_single_notification(&aggregator, notifier).await;
    }

    #[tokio::test]
    async fn test_notified_multiple_from_diff_threads() {
        let aggregator = NotifyAggregator::new();
        let notifier1 = Arc::new(Notify::new());
        let notifier2 = Arc::new(Notify::new());

        aggregator.add_notifier(notifier1.clone());
        aggregator.add_notifier(notifier2.clone());

        tokio::spawn(async move {
            time::sleep(Duration::from_millis(50)).await;
            notifier1.notify_waiters();
        });

        tokio::spawn(async move {
            time::sleep(Duration::from_millis(100)).await;
            notifier2.notify_waiters();
        });

        tokio::time::timeout(Duration::from_millis(500), aggregator.notified()).await.unwrap();
        tokio::time::timeout(Duration::from_millis(500), aggregator.notified()).await.unwrap();
    }

    #[tokio::test]
    async fn test_send_multiple_notifications_with_pause() {
        let aggregator = NotifyAggregator::new();
        let notifier1 = Arc::new(Notify::new());
        let notifier2 = Arc::new(Notify::new());

        aggregator.add_notifier(notifier1.clone());
        aggregator.add_notifier(notifier2.clone());

        // Add sleep to ensure aggregator is re-awaited between notifications
        tokio::spawn(async move {
            time::sleep(Duration::from_millis(20)).await;
            notifier1.notify_waiters();
            time::sleep(Duration::from_millis(20)).await;
            notifier2.notify_waiters();
        });

        tokio::time::timeout(Duration::from_millis(500), aggregator.notified()).await.unwrap();
        tokio::time::timeout(Duration::from_millis(500), aggregator.notified()).await.unwrap();
    }

    #[tokio::test]
    async fn test_multiple_notifications_count() {
        let aggregator = NotifyAggregator::new();
        let notifier1 = Arc::new(Notify::new());
        let notifier2 = Arc::new(Notify::new());

        aggregator.add_notifier(notifier1.clone());
        aggregator.add_notifier(notifier2.clone());

        let mut awaken_count = 0;

        tokio::spawn(async move {
            for _ in 0..5 {
                time::sleep(Duration::from_millis(30)).await;
                notifier1.notify_waiters();
            }
        });

        tokio::spawn(async move {
            for _ in 0..5 {
                time::sleep(Duration::from_millis(100)).await;
                notifier2.notify_waiters();
            }
        });

        for _ in 0..10 {
            tokio::time::timeout(Duration::from_millis(500), aggregator.notified()).await.unwrap();
            awaken_count += 1;
        }

        assert_eq!(awaken_count, 10);
    }

    #[tokio::test]
    async fn test_no_notification() {
        let aggregator = NotifyAggregator::new();
        let notifier1 = Arc::new(Notify::new());
        let notifier2 = Arc::new(Notify::new());

        aggregator.add_notifier(notifier1);
        aggregator.add_notifier(notifier2);

        // Use a timeout to test that no notifications are received
        let result = tokio::time::timeout(Duration::from_millis(500), aggregator.notified()).await;
        assert!(result.is_err()); // Timeout error expected
    }
}
