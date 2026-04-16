use std::thread;

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

use crate::config::{CLIPSYNC_SERVICE_TYPE, CLIPSYNC_WS_PORT};
use crate::domain::state::SharedState;
use crate::network::upsert_discovered_device;

pub fn start_mdns_discovery(state: SharedState, device_name: String) {
    let daemon = match ServiceDaemon::new() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("mDNS daemon init failed: {e}");
            return;
        }
    };

    let host_name = format!("{device_name}.local.");
    let service_info = match ServiceInfo::new(
        CLIPSYNC_SERVICE_TYPE,
        &device_name,
        &host_name,
        "",
        CLIPSYNC_WS_PORT,
        None,
    ) {
        Ok(info) => info,
        Err(e) => {
            eprintln!("mDNS service info creation failed: {e}");
            return;
        }
    };

    if let Err(e) = daemon.register(service_info) {
        eprintln!("mDNS register failed: {e}");
        return;
    }

    let receiver = match daemon.browse(CLIPSYNC_SERVICE_TYPE) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("mDNS browse failed: {e}");
            return;
        }
    };

    thread::spawn(move || {
        let _keep_alive = daemon;

        while let Ok(event) = receiver.recv() {
            if let ServiceEvent::ServiceResolved(info) = event {
                let resolved_name = info.get_fullname().to_string();
                if resolved_name.contains(&device_name) {
                    continue;
                }

                let first_addr = info.get_addresses().iter().next().map(|a| a.to_string());
                if let Some(ip) = first_addr {
                    let addr = format!("{}:{}", ip, info.get_port());
                    upsert_discovered_device(&state, resolved_name, addr);
                }
            }
        }
    });
}
