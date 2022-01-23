// Portions copyright (c) University of Delaware 1992-2015 under the NTP license
//
// Permission to use, copy, modify, and distribute this software and its documentation for any
// purpose with or without fee is hereby granted, provided that the above copyright notice appears
// in all copies and that both the copyright notice and this permission notice appear in supporting
// documentation, and that the name University of Delaware not be used in advertising or publicity
// pertaining to distribution of the software without specific, written prior permission. The
// University of Delaware makes no representations about the suitability this software for any
// purpose. It is provided "as is" without express or implied warranty.

use crate::pps::PPS;
use crate::timestamp::Timestamp;
use anyhow::anyhow;
use anyhow::Result;
use std::ops::Deref;
use tokio::sync::watch;
use tracing::error;

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

    pub async fn measure_precision(&self, pps: PPS) -> Result<i32> {
        let mut i = 0;
        let mut tick = self.measure_tick(pps).await?;

        while tick <= 1.0 {
            tick *= 2.0;
            i -= 1;
        }

        if tick - 1.0 > 1.0 - tick / 2.0 {
            i += 1;
        }

        Ok(i)
    }

    async fn measure_tick(&self, pps: PPS) -> Result<f64> {
        let mut current_timestamp = pps.current_timestamp();
        let mut tick = u32::MAX;
        let mut repeats: u32 = 0;
        let mut max_repeats: u32 = 0;
        let mut changes: u32 = 0;

        let mut last = if let Some(ts) = next_tick(&mut current_timestamp).await {
            ts.reference_nsec
        } else {
            return Err(anyhow!("Unable to retrieve timestamp"));
        };

        let mut loops = 0;

        while let Some(ts) = next_tick(&mut current_timestamp).await {
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

async fn next_tick(
    current_timestamp: &mut watch::Receiver<Option<Timestamp>>,
) -> Option<Timestamp> {
    if let Err(_) = current_timestamp.changed().await {
        error!("PPS source shut down");
        return None;
    }

    current_timestamp.borrow().deref().clone()
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
