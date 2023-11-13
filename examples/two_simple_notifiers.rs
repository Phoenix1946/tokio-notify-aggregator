use std::sync::Arc;
use tokio::sync::Notify;
use tokio_notify_aggregator::NotifyAggregator;

/// Creates two notifiers, one that triggers every second and one that triggers every 3 seconds.
/// Then creates an aggregator and registers the notifiers. Finally, it starts listening for
/// notifications from both the aggregator and the notifiers themselves.
#[tokio::main]
async fn main() {
    // Create a notifier & start triggering
    let notifier1sec = Arc::new(Notify::new());
    {
        let notifier1sec = notifier1sec.clone();
        tokio::spawn(async move {
            loop {
                println!("notifier1 notifying");
                notifier1sec.notify_waiters();
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
    }

    // Create another notifier & start triggering
    let notifier3secs = Arc::new(Notify::new());
    {
        let notifier3secs = notifier3secs.clone();
        tokio::spawn(async move {
            loop {
                println!("notifier2 notifying");
                notifier3secs.notify_waiters();
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        });
    }

    // Start listening for single notifications
    {
        let notifier1sec = notifier1sec.clone();
        tokio::spawn(async move {
            loop {
                notifier1sec.notified().await;
                println!("notifier1 notified");
            }
        });
    }
    {
        let notifier3secs = notifier3secs.clone();
        tokio::spawn(async move {
            loop {
                notifier3secs.notified().await;
                println!("notifier2 notified");
            }
        });
    }

    // Create an aggregator & register the notifiers
    let aggregator = NotifyAggregator::new();
    aggregator.add_notifier(notifier1sec);
    aggregator.add_notifier(notifier3secs);

    // Start listening for notifications
    loop {
        aggregator.notified().await;
        println!("AGGREGATOR NOTIFIED");
        println!("--------------------")
    }
}