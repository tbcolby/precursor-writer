use std::io::Write;
use std::net::TcpListener;
use usb_device_xous::UsbHid;

const EXPORT_PORT: u16 = 7879;
const DEFAULT_AUTOTYPE_DELAY_MS: usize = 30;

pub struct ExportSystem {
    tt: ticktimer_server::Ticktimer,
    usb_dev: UsbHid,
}

#[derive(Debug)]
pub enum ExportError {
    UsbNotConnected,
    TcpBindFailed,
    TcpAcceptFailed,
    TcpWriteFailed,
}

impl ExportSystem {
    pub fn new() -> Self {
        let tt = ticktimer_server::Ticktimer::new().unwrap();
        let usb_dev = UsbHid::new();
        // Set a reasonable default autotype delay
        usb_dev.set_autotype_delay_ms(DEFAULT_AUTOTYPE_DELAY_MS);
        Self { tt, usb_dev }
    }

    /// Set the delay between keystrokes during USB autotype (in milliseconds).
    /// Default is 30ms. Lower values type faster but may miss characters on some hosts.
    pub fn set_autotype_delay(&self, delay_ms: usize) {
        self.usb_dev.set_autotype_delay_ms(delay_ms);
    }

    /// Check if USB HID keyboard is available for autotype.
    pub fn is_usb_ready(&self) -> bool {
        // Try a quick check - if we can send an empty string, USB is connected
        self.usb_dev.send_str("").is_ok()
    }

    /// Export document content via TCP on port 7879.
    /// Blocks until a client connects and receives the data.
    pub fn export_tcp(&self, content: &str) -> Result<usize, ExportError> {
        log::info!("Starting TCP export on port {}", EXPORT_PORT);

        let listener = match TcpListener::bind(format!("0.0.0.0:{}", EXPORT_PORT)) {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to bind port {}: {:?}", EXPORT_PORT, e);
                return Err(ExportError::TcpBindFailed);
            }
        };

        // Wait for connection
        match listener.accept() {
            Ok((mut stream, addr)) => {
                log::info!("Export connection from {:?}", addr);
                let bytes = content.as_bytes();
                if let Err(e) = stream.write_all(bytes) {
                    log::error!("Failed to write export data: {:?}", e);
                    return Err(ExportError::TcpWriteFailed);
                }
                log::info!("Export complete: {} bytes sent", bytes.len());
                Ok(bytes.len())
            }
            Err(e) => {
                log::error!("Accept failed: {:?}", e);
                Err(ExportError::TcpAcceptFailed)
            }
        }
        // Listener drops and port is released
    }

    /// Export document content via USB keyboard autotype.
    /// Types each character as if typed on a USB keyboard.
    /// Returns the number of characters typed, or an error if USB is not connected.
    pub fn export_usb_autotype(&self, content: &str) -> Result<usize, ExportError> {
        log::info!("Starting USB autotype export: {} chars", content.len());

        match self.usb_dev.send_str(content) {
            Ok(sent) => {
                log::info!("USB autotype complete: {} chars typed", sent);
                Ok(sent)
            }
            Err(e) => {
                log::error!("USB autotype failed: {:?}", e);
                Err(ExportError::UsbNotConnected)
            }
        }
    }

    /// Export with progress callback for long documents.
    /// Useful for showing a progress indicator during export.
    pub fn export_usb_autotype_chunked<F>(
        &self,
        content: &str,
        chunk_size: usize,
        mut progress: F,
    ) -> Result<usize, ExportError>
    where
        F: FnMut(usize, usize), // (chars_sent, total_chars)
    {
        log::info!("Starting chunked USB autotype: {} chars", content.len());
        let total = content.len();
        let mut sent = 0;

        for chunk in content.as_bytes().chunks(chunk_size) {
            let chunk_str = match std::str::from_utf8(chunk) {
                Ok(s) => s,
                Err(_) => {
                    // Handle UTF-8 boundary issues by converting what we can
                    let s = String::from_utf8_lossy(chunk);
                    match self.usb_dev.send_str(&s) {
                        Ok(n) => {
                            sent += n;
                            progress(sent, total);
                            continue;
                        }
                        Err(e) => {
                            log::error!("USB autotype failed at char {}: {:?}", sent, e);
                            return Err(ExportError::UsbNotConnected);
                        }
                    }
                }
            };

            match self.usb_dev.send_str(chunk_str) {
                Ok(n) => {
                    sent += n;
                    progress(sent, total);
                }
                Err(e) => {
                    log::error!("USB autotype failed at char {}: {:?}", sent, e);
                    return Err(ExportError::UsbNotConnected);
                }
            }

            // Small pause between chunks to prevent buffer overflow
            self.tt.sleep_ms(50).ok();
        }

        log::info!("Chunked USB autotype complete: {} chars typed", sent);
        Ok(sent)
    }
}
