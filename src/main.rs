// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

#![allow(unused)]

use std::collections::HashMap;
use std::vec;

use btleplug::api::{Central, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use std::time::Duration;
use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder},
};
use tokio::runtime::Runtime;
use tray_icon::{
    menu::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, TrayIconBuilder, TrayIconEvent,
};
use windows::core::{Error, RuntimeType, HSTRING};
use windows::Devices::Bluetooth::GenericAttributeProfile::GattSession;
use windows::Devices::Bluetooth::{BluetoothDevice, BluetoothLEDevice};
use windows::Devices::Enumeration::DeviceInformation;
use windows::Foundation::TypedEventHandler;
use windows_future::IAsyncOperation;

enum UserEvent {
    TrayIconEvent(tray_icon::TrayIconEvent),
    MenuEvent(tray_icon::menu::MenuEvent),
}

#[tokio::main]
async fn main() {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let rt = Runtime::new().unwrap();

    // set a tray event handler that forwards the event and wakes up the event loop
    let proxy = event_loop.create_proxy();
    TrayIconEvent::set_event_handler(Some(move |event| {
        proxy.send_event(UserEvent::TrayIconEvent(event));
    }));

    // set a menu event handler that forwards the event and wakes up the event loop
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        proxy.send_event(UserEvent::MenuEvent(event));
    }));

    let tray_menu = Menu::new();
    let quit_i = MenuItem::new("Quit", true, None);

    // Get Bluetooth devices
    let bluetooth_devices = get_paired_bluetooth_devices().await.unwrap();

    // Store device info mapped to menu items
    let mut device_map = HashMap::new();
    let device_items: Vec<MenuItem> = bluetooth_devices
        .iter()
        .map(|device_info| {
            let item = MenuItem::new(
                device_info
                    .Name()
                    .expect("device name doesn't exist")
                    .to_string(),
                true,
                None,
            );
            device_map.insert(item.id().clone(), device_info.Id().unwrap());
            item
        })
        .collect();

    tray_menu.append_items(&[
        &PredefinedMenuItem::about(
            None,
            Some(AboutMetadata {
                name: Some("Bluetooth Tray".to_string()),
                copyright: Some("Copyright bluetray".to_string()),
                ..Default::default()
            }),
        ),
        &PredefinedMenuItem::separator(),
    ]);

    // Add device menu items
    for item in &device_items {
        tray_menu.append(item);
    }

    tray_menu.append(&PredefinedMenuItem::separator());
    tray_menu.append(&quit_i);

    let mut tray_icon = None;

    let menu_channel = MenuEvent::receiver();
    let tray_channel = TrayIconEvent::receiver();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(tao::event::StartCause::Init) => {
                let icon = Icon::from_rgba(vec![200, 200, 0, 0], 1, 1).unwrap();

                // We create the icon once the event loop is actually running
                // to prevent issues like https://github.com/tauri-apps/tray-icon/issues/90
                tray_icon = Some(
                    TrayIconBuilder::new()
                        .with_menu(Box::new(tray_menu.clone()))
                        .with_tooltip("tao - awesome windowing lib")
                        .with_icon(icon)
                        .build()
                        .unwrap(),
                );

                // We have to request a redraw here to have the icon actually show up.
                // Tao only exposes a redraw method on the Window so we use core-foundation directly.
                #[cfg(target_os = "macos")]
                unsafe {
                    use objc2_core_foundation::{CFRunLoopGetMain, CFRunLoopWakeUp};

                    let rl = CFRunLoopGetMain().unwrap();
                    CFRunLoopWakeUp(&rl);
                }
            }

            Event::UserEvent(UserEvent::TrayIconEvent(event)) => {
                println!("{event:?}");
            }

            Event::UserEvent(UserEvent::MenuEvent(event)) => {
                println!("{event:?}");

                if event.id == quit_i.id() {
                    tray_icon.take();
                    *control_flow = ControlFlow::Exit;
                } else if let Some(device_id) = device_map.get(&event.id) {
                    if let Err(e) = connect_to_bluetooth_device(device_id) {
                        println!("Failed to connect to device: {}", e);
                    }
                }
            }

            _ => {}
        }
    })
}

async fn get_paired_bluetooth_devices() -> Result<Vec<DeviceInformation>, Error> {
    // Define a query for paired Bluetooth devices
    let selector = BluetoothDevice::GetDeviceSelectorFromPairingState(true)?;

    // Find all paired devices
    let devices_operation = DeviceInformation::FindAllAsyncAqsFilter(&selector)?;
    let devices: Vec<_> = devices_operation.get()?.into_iter().collect();

    // let mut device_names = Vec::new();

    // Iterate through each device
    // for device in devices {
    //     if let Ok(name) = device.Name() {
    //         device_names.push(name.to_string());
    //     }
    // }

    Ok(devices)
}

fn connect_to_bluetooth_device(device_id: &HSTRING) -> Result<(), Error> {
    // Create connection operation
    let device = BluetoothLEDevice::FromIdAsync(device_id)?.get()?;
    let gatt_services = device.GetGattServicesAsync()?.get()?;
    let session = GattSession::FromDeviceIdAsync(&device.BluetoothDeviceId()?)?.get()?;
    session.SetMaintainConnection(true);
    Ok(())
}
