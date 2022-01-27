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

const MIN_CHANGES: u32 = 12;
const MIN_CLOCK_INCREMENT: u32 = 86;

pub struct Precision {}

impl Precision {
    pub fn new() -> Self {
        Precision {}
    }

    /// Calculate precision for +pps+.
    ///
    /// This will capture up to `MIN_CHANGES` samples and calculate the measurement precision for
    /// that PPS device.
    pub async fn once(&self, pps: PPS) -> Result<i32> {
        let (sender, mut receiver) = watch::channel(0.0);

        let task = tokio::spawn(measure_ticks(pps, sender));

        receiver.changed().await?;
        task.abort();

        let tick = *receiver.borrow().deref();

        Ok(precision(tick))
    }

    /// Continuously measure precision for +pps+.
    ///
    /// This will start updating precision through the returned `watch::Receiver` after
    /// `MIN_CHANGES` samples
    pub async fn watch(&self, pps: PPS) -> watch::Receiver<i32> {
        let (tick_sender, mut tick_receiver) = watch::channel(0.0);

        tokio::spawn(measure_ticks(pps, tick_sender));

        let (precision_sender, precision_receiver) = watch::channel(0);

        tokio::spawn(async move {
            while let Ok(_) = tick_receiver.changed().await {
                let tick = *tick_receiver.borrow().deref();

                if let Err(_) = precision_sender.send(precision(tick)) {
                    break;
                }
            }
        });

        precision_receiver
    }
}

async fn measure_ticks(pps: PPS, tick_times: watch::Sender<f64>) -> Result<()> {
    let mut tick = u32::MAX;
    let mut repeats = 0;
    let mut max_repeats = 0;
    let mut changes = 0;

    let mut current_timestamp = pps.current_timestamp();

    let mut last = next_tick(&mut current_timestamp).await?.reference_nsec;

    loop {
        let val = next_tick(&mut current_timestamp).await?.reference_nsec;
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
}

fn precision(mut tick: f64) -> i32 {
    let mut precision = 0;

    while tick <= 1.0 {
        tick *= 2.0;
        precision -= 1;
    }

    if tick - 1.0 > 1.0 - tick / 2.0 {
        precision += 1;
    }

    precision
}

async fn next_tick(current_timestamp: &mut watch::Receiver<Timestamp>) -> Result<Timestamp> {
    current_timestamp.changed().await?;

    Ok(current_timestamp.borrow().deref().clone())
}
