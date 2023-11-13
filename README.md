# Tokio Notify Aggregator

`tokio_notify_aggregator` is a Rust library providing a mechanism to aggregate multiple Tokio [`Notify`] instances 
into a single source of notifications. This allows for efficiently waiting on notifications from any of a set 
of [`Notify`] instances.

[`Notify`]: https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html

## Features

- Aggregate multiple `Notify` instances into a single stream.
- Mimics the API of `Notify` for ease of use.

## Installation

Add `tokio_notify_aggregator` to your `Cargo.toml`:

```toml
[dependencies]
tokio_notify_aggregator = "*"  # Use the latest version
```


## Usage
Once [`Notify`] instances have been added to the aggregator, the aggregator can be used to wait for notifications
in the same manner as a single [`Notify`] instance.
The below (slightly verbose) example demonstrates this - three [`Notify`] instances are added to the aggregator and
triggered at different times. The aggregator is then used to wait for notifications from any of the instances.
```rust no_run
use tokio::sync::Notify;
use tokio_notify_aggregator::NotifyAggregator;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let notify1 = Arc::new(Notify::new());
    let notify2 = Arc::new(Notify::new());
    let notify3 = Arc::new(Notify::new());

    let mut aggregator = NotifyAggregator::new();
    aggregator.add_notifier(notify1.clone());
    aggregator.add_notifier(notify2.clone());
    aggregator.add_notifier(notify3.clone());

    // Notify all three instances
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        notify1.notify_waiters();
    });
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        notify2.notify_waiters();
    });
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        notify3.notify_waiters();
    });

    // Wait for notifications
    aggregator.notified().await;
    println!("Notified");
    aggregator.notified().await;
    println!("Notified");
    aggregator.notified().await;
    println!("Notified");
    match tokio::time::timeout(Duration::from_millis(500), aggregator.notified()).await {
        Ok(_) => panic!("Should not have been notified"),
        Err(_) => println!("Timed out"),
    }
}
```

Aggregators can be constructed in several ways:
```rust no_run
use tokio_notify_aggregator::NotifyAggregator;
use tokio::sync::Notify;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Construct an aggregator with a single Notify instance
    let notify = Arc::new(Notify::new());
    let aggregator = NotifyAggregator::new();
    aggregator.add_notifier(notify);

    // Construct an aggregator from a vector of Notify instances
    let notify1 = Arc::new(Notify::new());
    let notify2 = Arc::new(Notify::new());
    let notify3 = Arc::new(Notify::new());
    let aggregator = NotifyAggregator::from_vec(&vec![notify1, notify2, notify3]);

    // Construct an aggregator from any iterator of Notify instances
    let notify1 = Arc::new(Notify::new());
    let notify2 = Arc::new(Notify::new());
    let notify3 = Arc::new(Notify::new());
    let aggregator = NotifyAggregator::from_iter([notify1, notify2, notify3]);
    
    // Construct a temporary aggregator
    let notify1 = Arc::new(Notify::new());
    let notify2 = Arc::new(Notify::new());
    NotifyAggregator::new()
        .add_notifier(notify1)
        .add_notifier(notify2)
        .notified()
        .await; // Will wait for notification from notify1 or notify2
}
```

## More Examples
- Two Simple Notifiers: [Example here](./examples/multiple_simultaneous_notifiers.rs)

- Multiple Simultaneous Notifiers: [Example here](./examples/two_simple_notifiers.rs)


For more detailed usage and examples, refer to the documentation.