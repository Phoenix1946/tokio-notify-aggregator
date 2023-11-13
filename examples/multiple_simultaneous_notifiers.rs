use std::sync::Arc;
use tokio::sync::Notify;
use tokio_notify_aggregator::NotifyAggregator;

static NOTIFIER_COUNT: usize = 10;

/// Creates a Vec of notifiers, each of which triggers every 3 seconds. Then creates an aggregator
/// and registers the notifiers. Finally, it starts listening for notifications from both the
/// aggregator and the notifiers themselves.
/// When the aggregator is notified, it will print a message to stdout and sleep for 20ms to mimic
/// some work being done. The notifiers which did not trigger the aggregator *should* all notify their
/// waiters before this sleep completes and the aggregator is re-awaited.
/// This behaviour should be the same as awaiting a single Notify which fires several times in
/// quick succession, before the client calls notified().await again
#[tokio::main]
async fn main() {
    // Create multiple notifiers
    let notifiers = (0..NOTIFIER_COUNT).map(|_| Arc::new(Notify::new())).collect::<Vec<_>>();

    // Register the notifiers with an aggregator & start listening for notifications
    let aggregator = NotifyAggregator::from(&notifiers);
    tokio::spawn(async move {
        loop {
            aggregator.notified().await;
            // Simulate some work to mimic a small delay between re-awaiting the aggregator
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            println!("AGGREGATOR NOTIFIED");
            println!("--------------------");
        }
    });

    // Start triggering the notifiers
    for i in 0..NOTIFIER_COUNT {
        let notifier = notifiers[i].clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                println!("notifier{} notifying", i);
                notifier.notify_waiters();
            }
        });
    }

    tokio::signal::ctrl_c().await.unwrap();
}