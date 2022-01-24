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

const MIN_CHANGES: u32 = 12;
const MIN_CLOCK_INCREMENT: u32 = 86;

pub struct Precision {}

impl Precision {
    pub fn new() -> Self {
        Precision {}
    }

    pub async fn measure(&self, pps: PPS) -> Result<i32> {
        let (sender, mut receiver) = watch::channel(0.0);

        let task = tokio::spawn(measure_ticks(pps, sender));

        receiver.changed().await?;
        task.abort();

        let mut tick = *receiver.borrow().deref();
        let mut i = 0;

        while tick <= 1.0 {
            tick *= 2.0;
            i -= 1;
        }

        if tick - 1.0 > 1.0 - tick / 2.0 {
            i += 1;
        }

        Ok(i)
    }
}

async fn measure_ticks(pps: PPS, tick_times: watch::Sender<f64>) -> Result<()> {
    let mut tick = u32::MAX;
    let mut repeats = 0;
    let mut max_repeats = 0;
    let mut changes = 0;

    let mut current_timestamp = pps.current_timestamp();

    let mut last = if let Some(ts) = next_tick(&mut current_timestamp).await {
        ts.reference_nsec
    } else {
        return Err(anyhow!("Unable to retrieve timestamp"));
    };

    while let Some(ts) = next_tick(&mut current_timestamp).await {
        let val = ts.reference_nsec;
        // We can use abs_diff() in the future
        let diff = if val < last { last - val } else { val - last };
        last = val;

        if diff > MIN_CLOCK_INCREMENT {
            max_repeats = repeats.max(max_repeats);
            repeats = 0;
            changes += 1;
            tick = diff.min(tick);
        } else {
            repeats += 1
        }

        if changes > MIN_CHANGES {
            if let Err(_) = tick_times.send(tick as f64 / u32::MAX as f64) {
                return Err(anyhow!("No longer measuring ticks"));
            }
        }
    }

    Err(anyhow!("Unable to retrieve timestamp"))
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
