use crate::{
    configuration::GpsConfig, device::Device, gps::GPS, gpsd::Response, pps::PPS,
    precision::Precision, shm::NtpShm,
};
use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::{broadcast, watch};
use tracing::info;

pub struct Devices {
    devices: HashMap<String, Device>,
}

impl Devices {
    pub async fn start(device_configuration: &Vec<GpsConfig>) -> Result<Self> {
        let mut devices = HashMap::new();

        create_devices(&mut devices, device_configuration).await?;

        for device in devices.values() {
            device.start();
        }

        Ok(Devices { devices })
    }

    pub fn devices(&self) -> Vec<&Device> {
        self.devices.values().collect()
    }

    pub fn gps_rx_for(&self, gps_name: String) -> Option<broadcast::Receiver<Response>> {
        if let Some(Device::GPS(gps)) = self.devices.get(&gps_name) {
            Some(gps.gpsd_tx.subscribe())
        } else {
            None
        }
    }

    pub fn pps_rx_for(&self, pps_name: String) -> Option<(PPS, watch::Receiver<i32>)> {
        if let Some(Device::PPS(pps, precision)) = self.devices.get(&pps_name) {
            Some((pps.clone(), precision.clone()))
        } else {
            None
        }
    }

    pub fn gps_devices(&self) -> Vec<&GPS> {
        self.devices
            .values()
            .filter_map(|d| {
                if let Device::GPS(gps) = d {
                    Some(gps)
                } else {
                    None
                }
            })
            .collect()
    }
}

async fn create_devices(
    devices: &mut HashMap<String, Device>,
    device_configuration: &Vec<GpsConfig>,
) -> Result<()> {
    for gps_config in device_configuration {
        create_device(devices, &gps_config).await?;
    }

    Ok(())
}

async fn create_device(
    devices: &mut HashMap<String, Device>,
    gps_config: &GpsConfig,
) -> Result<()> {
    let gps = GPS::new(gps_config).await?;

    info!("registered GPS {} ({})", gps_config.name, gps_config.device);

    if let Some(ntp_unit) = gps_config.ntp_unit {
        let mut rx = gps.ntp_tx.subscribe();
        let local = tokio::task::LocalSet::new();

        local.spawn_local(async move {
            let mut ntp_shm = NtpShm::new(ntp_unit).unwrap();

            while let Ok(ts) = rx.recv().await {
                ntp_shm.update_old(-1, 0, &ts);
            }
        });

        info!(
            "Sending GPS time from {} via NTP unit {}",
            gps_config.name, ntp_unit
        );
    }

    devices.insert(gps_config.name.clone(), Device::GPS(gps));

    if let Some(pps_config) = &gps_config.pps {
        let pps_name = pps_config.device.clone();

        let pps = PPS::new(pps_name.clone()).unwrap();
        let precision = Precision::new().watch(pps.clone()).await;

        if let Some(ntp_unit) = pps_config.ntp_unit {
            let mut current_timestamp = pps.current_timestamp();
            let ntp_precision = precision.clone();
            let local = tokio::task::LocalSet::new();

            local.spawn_local(async move {
                let mut ntp_shm = NtpShm::new(ntp_unit).unwrap();

                loop {
                    ntp_shm
                        .update(&ntp_precision, 0, &mut current_timestamp)
                        .await;
                }
            });

            info!(
                "Sending PPS time from {} via NTP unit {}",
                &pps_name, ntp_unit
            );
        }

        devices.insert(pps_name.clone(), Device::PPS(pps, precision));

        info!("registered PPS {}", &pps_name);
    };

    Ok(())
}
