//! Discover Bluetooth devices and list them.

use bluer::{
    AdapterEvent, Device, DeviceEvent, DeviceProperty,
};
use futures::{pin_mut, stream::SelectAll, StreamExt};
use notify_rust::Notification;

async fn notify(device: Device, connected: bool) -> bluer::Result<()> {
    let status = if connected {
        "Device connected"
    } else {
        "Device disconnected"
    };
    Notification::new()
    .summary(status)
    .body(format!("{} ({})", device.name().await.unwrap().unwrap(), device.address()).as_str())
    .icon("bluetooth")
    .timeout(5)
    .show().unwrap();
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    env_logger::init();
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    println!(
        "Discovering devices using Bluetooth adapter {}\n",
        adapter.name()
    );
    adapter.set_powered(true).await?;

    let device_events = adapter.discover_devices().await?;
    pin_mut!(device_events);

    let mut all_change_events = SelectAll::new();

    loop {
        tokio::select! {
            Some(device_event) = device_events.next() => {
                match device_event {
                    AdapterEvent::DeviceAdded(addr) => {
                        let device = adapter.device(addr)?;
                        let change_events = device.events().await?.map(move |evt| (addr, evt));
                        all_change_events.push(change_events);

                        let connected = device.is_connected().await?;
                        if connected {
                            println!("Device added and connected: {addr}");
                            notify(device, true).await?;
                        }
                    }
                    _ => (),
                }
            }
            Some((addr, DeviceEvent::PropertyChanged(property))) = all_change_events.next() => {
                    match property {
                        DeviceProperty::Connected(value) => {
                            println!("Device {addr} changed connected property to {value}");
                            let device = adapter.device(addr)?;
                            notify(device, value).await?;
                        }
                        _ => {}
                    }
            }
            else => break
        }
    }

    Ok(())
}
