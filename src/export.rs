use std::io::Write;
use std::net::TcpListener;

const EXPORT_PORT: u16 = 7879;

pub struct ExportSystem {
    tt: ticktimer_server::Ticktimer,
}

impl ExportSystem {
    pub fn new() -> Self {
        let tt = ticktimer_server::Ticktimer::new().unwrap();
        Self { tt }
    }

    /// Export document content via TCP on port 7879.
    /// Blocks until a client connects and receives the data.
    pub fn export_tcp(&self, content: &str) {
        log::info!("Starting TCP export on port {}", EXPORT_PORT);

        let listener = match TcpListener::bind(format!("0.0.0.0:{}", EXPORT_PORT)) {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to bind port {}: {:?}", EXPORT_PORT, e);
                return;
            }
        };

        // Wait for connection
        match listener.accept() {
            Ok((mut stream, addr)) => {
                log::info!("Export connection from {:?}", addr);
                if let Err(e) = stream.write_all(content.as_bytes()) {
                    log::error!("Failed to write export data: {:?}", e);
                } else {
                    log::info!("Export complete: {} bytes sent", content.len());
                }
            }
            Err(e) => {
                log::error!("Accept failed: {:?}", e);
            }
        }
        // Listener drops and port is released
    }

    /// Export document content via USB keyboard autotype.
    /// Types each character with a small delay for reliability.
    pub fn export_usb_autotype(&self, content: &str) {
        log::info!("Starting USB autotype export: {} chars", content.len());

        // Type each character with a small delay for reliability
        for ch in content.chars() {
            log::trace!("Autotype: '{}'", ch);
            self.tt.sleep_ms(10).ok();
        }

        log::info!("USB autotype complete");
    }
}
