use crate::timestamp::Timestamp;
use anyhow::anyhow;
use anyhow::Result;
use std::marker::Unpin;
use tokio_stream::Stream;
use tokio_stream::StreamExt;

pub struct Precision {
    /// Maximum number of samples to process when measuring ticks
    max_samples: u32,
    /// Minimum number of sample-to-sample changes when measuring ticks
    min_changes: u32,
    /// Minimum clock increment in 32 bit fractional seconds
    min_clock_increment: u32,
}

impl Precision {
    pub fn new(max_samples: u32, min_changes: u32, min_clock_increment: u32) -> Self {
        Precision {
            max_samples,
            min_changes,
            min_clock_increment,
        }
    }

    pub async fn measure_precision<T>(&self, stream: T) -> Result<i32>
    where
        T: Stream<Item = Timestamp> + Unpin,
    {
        let mut i = 0;
        let mut tick = self.measure_tick(stream).await?;

        while tick <= 1.0 {
            tick *= 2.0;
            i -= 1;
        }

        if tick - 1.0 > 1.0 - tick / 2.0 {
            i += 1;
        }

        Ok(i)
    }

    async fn measure_tick<T>(&self, mut stream: T) -> Result<f64>
    where
        T: Stream<Item = Timestamp> + Unpin,
    {
        let mut tick = u32::MAX;
        let mut repeats: u32 = 0;
        let mut max_repeats: u32 = 0;
        let mut changes: u32 = 0;

        let mut last = if let Some(ts) = stream.next().await {
            ts.reference_nsec
        } else {
            return Err(anyhow!("Unable to retrieve timestamp"));
        };

        let mut loops = 0;

        while let Some(ts) = stream.next().await {
            let val = ts.reference_nsec;
            let diff = val - last;
            last = val;

            if diff > self.min_clock_increment {
                max_repeats = repeats.max(max_repeats);
                repeats = 0;
                changes += 1;
                tick = diff.min(tick);
            } else {
                repeats += 1
            }

            loops += 1;

            if loops > self.max_samples || changes > self.min_changes {
                break;
            }
        }

        Ok(tick as f64 / u32::MAX as f64)
    }
}

impl Default for Precision {
    fn default() -> Self {
        Precision {
            max_samples: 60,
            min_changes: 12,
            min_clock_increment: 86,
        }
    }
}
