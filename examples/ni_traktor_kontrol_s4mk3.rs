// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

use std::ops::Not as _;

use djio::{HidApi, HidUsagePage, devices::ni_traktor_kontrol_s4mk3};

fn main() {
    pretty_env_logger::init();

    match run() {
        Ok(()) => (),
        Err(err) => log::error!("{err}"),
    }
}

fn run() -> anyhow::Result<()> {
    log::info!("Initializing HID API");
    let mut api = HidApi::new()?;

    log::info!("Querying HID devices");
    let devices = api.query_devices_dedup()?;
    log::info!(
        "Found {num_devices} HID device(s)",
        num_devices = devices.len()
    );

    log::info!("Filtering HID devices by usage page");
    let devices = devices
        .into_iter()
        .filter(|device| {
            matches!(
                HidUsagePage::from(device.info().usage_page()),
                HidUsagePage::VendorDefined(_)
            )
        })
        .collect::<Vec<_>>();
    if devices.is_empty() {
        log::warn!("Found no suitable HID device");
        return Ok(());
    }
    log::info!(
        "Found {num_devices} suitable HID device(s)",
        num_devices = devices.len()
    );

    let mut device_context = None;
    for mut device in devices {
        let device_info = device.info().clone();
        log::info!(
            "Found HID device {manufacturer_name} {product_name}: path = {path}, vid = \
             0x{vid:0.4x}, pid = 0x{pid:0.4x}, sn = '{sn}', usage = {usage}, usage_page = \
             {usage_page:?}, release_number = {release_number}, interface_number = \
             {interface_number}",
            manufacturer_name = device_info
                .manufacturer_string()
                .and_then(|s| s.trim().is_empty().not().then_some(s))
                .unwrap_or("(no manufacturer name)"),
            product_name = device_info
                .product_string()
                .and_then(|s| s.trim().is_empty().not().then_some(s))
                .unwrap_or("(no product name)"),
            path = device_info.path().to_str().unwrap_or_default(),
            vid = device_info.vendor_id(),
            pid = device_info.product_id(),
            sn = device_info.serial_number().unwrap_or_default(),
            usage = device_info.usage(),
            usage_page = HidUsagePage::from(device_info.usage_page()),
            release_number = device_info.release_number(),
            interface_number = device_info.interface_number(),
        );
        if let Err(err) = device.connect(&api) {
            log::warn!("Failed to connect device {device_info:?}: {err}");
            continue;
        }
        if !ni_traktor_kontrol_s4mk3::DeviceContext::is_supported(&device_info) {
            log::info!("Ignoring unsupported device {device_info:?}");
            continue;
        }
        debug_assert!(
            device_context.is_none(),
            "only a single device is supported"
        );
        let mut new_device_context = ni_traktor_kontrol_s4mk3::DeviceContext::attach(device)?;
        log::info!(
            "Initializing device: {device_info:?}",
            device_info = new_device_context.info()
        );
        new_device_context.initialize();
        device_context = Some(new_device_context);
    }

    log::info!("TODO: Run event loop");

    if let Some(mut device_context) = device_context {
        log::info!(
            "Finalizing device: {device_info:?}",
            device_info = device_context.info()
        );
        device_context.finalize();
    }

    Ok(())
}
