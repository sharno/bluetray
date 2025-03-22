use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::vec;

use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder},
};
use tray_icon::{
    menu::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, TrayIconBuilder, TrayIconEvent,
};
use windows::{core::{Error, HSTRING}, Networking::Sockets::StreamSocket};
use windows::Devices::Bluetooth::BluetoothDevice;
use windows::Devices::Enumeration::DeviceInformation;

enum UserEvent {
    TrayIconEvent(tray_icon::TrayIconEvent),
    MenuEvent(tray_icon::menu::MenuEvent),
}

// This struct will manage active Bluetooth connections
struct ConnectionManager {
    active_connections: HashMap<String, StreamSocket>,
}

impl ConnectionManager {
    fn new() -> Self {
        Self {
            active_connections: HashMap::new(),
        }
    }

    fn connect_device(&mut self, device_id: &HSTRING) -> Result<(), Error> {
        let device_id_str = device_id.to_string();
        
        // Check if already connected
        if self.active_connections.contains_key(&device_id_str) {
            println!("Device already connected: {}", device_id_str);
            return Ok(());
        }
        
        // Connect to the device
        let socket = connect_to_bluetooth_device(device_id)?;
        
        // Store the connection
        self.active_connections.insert(device_id_str, socket);
        println!("Connection stored. Active connections: {}", self.active_connections.len());
        
        Ok(())
    }

    fn disconnect_device(&mut self, device_id: &str) -> bool {
        self.active_connections.remove(device_id).is_some()
    }
}

#[tokio::main]
async fn main() {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    // Create connection manager
    let connection_manager = Arc::new(Mutex::new(ConnectionManager::new()));

    // set a tray event handler that forwards the event and wakes up the event loop
    let proxy = event_loop.create_proxy();
    TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::TrayIconEvent(event));
    }));

    // set a menu event handler that forwards the event and wakes up the event loop
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::MenuEvent(event));
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
    ]).unwrap();

    // Add device menu items
    for item in &device_items {
        tray_menu.append(item).unwrap();
    }

    tray_menu.append(&PredefinedMenuItem::separator()).unwrap();
    tray_menu.append(&quit_i).unwrap();

    let mut tray_icon = None;

    let connection_manager_clone = connection_manager.clone();
    
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
                    // Use connection manager to connect to the device
                    let mut manager = connection_manager_clone.lock().unwrap();
                    if let Err(e) = manager.connect_device(device_id) {
                        println!("Failed to connect to device: {}", e);
                    }
                }
            }

            _ => {}
        }
    })
}

async fn get_paired_bluetooth_devices() -> Result<Vec<DeviceInformation>, Error> {
    let selector = BluetoothDevice::GetDeviceSelectorFromPairingState(true)?;
    let devices_operation = DeviceInformation::FindAllAsyncAqsFilter(&selector)?;
    let devices: Vec<_> = devices_operation.get()?.into_iter().collect();

    Ok(devices)
}

fn connect_to_bluetooth_device(device_id: &HSTRING) -> Result<StreamSocket, Error> {
    println!("Attempting to connect to device with ID: {:?}", device_id);
    let device = BluetoothDevice::FromIdAsync(device_id)?.get()?;
    let service = device.GetRfcommServicesAsync()?.get()?.Services()?.GetAt(0)?;
    let socket = StreamSocket::new()?;
    println!("Connecting to device: {:?}, {:?}", service.ConnectionHostName()?.ToString()?, service.ConnectionServiceName()?);
    let _connection = socket.ConnectAsync(
        &service.ConnectionHostName()?, 
        &service.ConnectionServiceName()?)?.get()?;
    println!("Connected to device: {:?}", device.Name()?);
    
    Ok(socket)
}
