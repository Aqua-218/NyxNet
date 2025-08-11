#![forbid(unsafe_code)]

//! Timing obfuscator queue.
//! Adds ±sigma randomized delay before releasing packets to mitigate timing analysis.

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use tokio::{sync::mpsc, time::{sleep, Duration}};

/// Obfuscated packet with payload bytes.
#[derive(Debug)]
pub struct Packet(pub Vec<u8>);

/// TimingObfuscator parameters.
pub struct TimingConfig {
    /// Mean delay in milliseconds.
    pub mean_ms: f64,
    /// Standard deviation.
    pub sigma_ms: f64,
}

impl Default for TimingConfig {
    fn default() -> Self {
        Self { mean_ms: 20.0, sigma_ms: 10.0 }
    }
}

/// Queue that releases packets after randomized delay.
pub struct TimingObfuscator {
    tx: mpsc::Sender<Packet>,
    rx: mpsc::Receiver<Packet>,
}

impl TimingObfuscator {
    pub fn new(config: TimingConfig) -> Self {
        let (int_tx, mut int_rx) = mpsc::channel::<Packet>(1024);
        let (out_tx, out_rx) = mpsc::channel::<Packet>(1024);
        // spawn worker
        tokio::spawn(async move {
            let mut rng = StdRng::from_entropy();
            while let Some(pkt) = int_rx.recv().await {
                // Sample delay: mean + N(0, sigma)
                let noise: f64 = rng.sample(rand_distr::Normal::new(0.0, config.sigma_ms).unwrap());
                let delay_ms = (config.mean_ms + noise).max(0.0);
                sleep(Duration::from_millis(delay_ms as u64)).await;
                if out_tx.send(pkt).await.is_err() {
                    break;
                }
            }
        });
        Self { tx: int_tx, rx: out_rx }
    }

    /// Enqueue packet for delayed release.
    pub async fn enqueue(&self, data: Vec<u8>) {
        let _ = self.tx.send(Packet(data)).await;
    }

    /// Receive next obfuscated packet.
    pub async fn recv(&mut self) -> Option<Packet> {
        self.rx.recv().await
    }

    /// Get a clone of the internal sender for enqueueing packets from other tasks.
    pub fn sender(&self) -> mpsc::Sender<Packet> {
        self.tx.clone()
    }
} 

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Instant, Duration};

    #[tokio::test]
    async fn delays_within_expected_distribution() {
        let config = TimingConfig { mean_ms: 15.0, sigma_ms: 5.0 };
        let obf = TimingObfuscator::new(config);
        let start = Instant::now();
        obf.enqueue(vec![1,2,3]).await;
        let mut rx = obf;
        let _pkt = rx.recv().await.expect("packet");
        let elapsed = start.elapsed().as_millis();
        // Expect delay roughly within 0..= (mean + 3*sigma) ≈ 30ms (wide tolerance for CI variance)
        assert!(elapsed <= 60, "elapsed {}ms too large", elapsed);
    }
}