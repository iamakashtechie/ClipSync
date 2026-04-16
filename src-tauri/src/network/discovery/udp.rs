use std::net::UdpSocket;
use std::thread;
use std::time::{Duration, Instant};

use crate::config::{CLIPSYNC_UDP_DISCOVERY_PORT, CLIPSYNC_WS_PORT};
use crate::domain::models::UdpDiscoveryBeacon;
use crate::domain::state::SharedState;
use crate::network::upsert_discovered_device;

pub fn start_udp_fallback_discovery(state: SharedState, device_name: String) {
    thread::spawn(move || {
        let socket = match UdpSocket::bind(("0.0.0.0", CLIPSYNC_UDP_DISCOVERY_PORT)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("UDP discovery bind failed: {e}");
                return;
            }
        };

        if let Err(e) = socket.set_broadcast(true) {
            eprintln!("UDP discovery broadcast setup failed: {e}");
            return;
        }
        if let Err(e) = socket.set_read_timeout(Some(Duration::from_secs(1))) {
            eprintln!("UDP discovery read timeout setup failed: {e}");
            return;
        }

        let beacon = UdpDiscoveryBeacon {
            name: device_name.clone(),
            ws_port: CLIPSYNC_WS_PORT,
            is_reply: false,
        };
        let beacon_payload = match serde_json::to_vec(&beacon) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("UDP discovery beacon encoding failed: {e}");
                return;
            }
        };

        let mut last_broadcast = Instant::now() - Duration::from_secs(10);
        let mut recv_buf = [0u8; 1024];

        loop {
            if last_broadcast.elapsed() >= Duration::from_secs(5) {
                let _ = socket.send_to(
                    &beacon_payload,
                    ("255.255.255.255", CLIPSYNC_UDP_DISCOVERY_PORT),
                );
                last_broadcast = Instant::now();
            }

            if let Ok((len, src_addr)) = socket.recv_from(&mut recv_buf) {
                if let Ok(remote) = serde_json::from_slice::<UdpDiscoveryBeacon>(&recv_buf[..len]) {
                    if remote.name == device_name {
                        continue;
                    }

                    let peer_addr = format!("{}:{}", src_addr.ip(), remote.ws_port);
                    upsert_discovered_device(&state, remote.name, peer_addr);

                    if !remote.is_reply {
                        let reply = UdpDiscoveryBeacon {
                            name: device_name.clone(),
                            ws_port: CLIPSYNC_WS_PORT,
                            is_reply: true,
                        };

                        if let Ok(reply_payload) = serde_json::to_vec(&reply) {
                            let _ = socket.send_to(
                                &reply_payload,
                                (src_addr.ip(), CLIPSYNC_UDP_DISCOVERY_PORT),
                            );
                        }
                    }
                }
            }
        }
    });
}
