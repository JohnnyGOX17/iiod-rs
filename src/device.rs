use std::time::Duration;

use crate::error::{Error, Result};
use crate::protocol;
use crate::transport::{TcpTransport, Transport};

/// An attribute on a device or channel.
#[derive(Debug, Clone)]
pub struct Attr {
    pub name: String,
    pub filename: String,
}

/// A single IIO channel (e.g. `voltage0`).
#[derive(Debug, Clone)]
pub struct Channel {
    pub id: String,
    pub name: Option<String>,
    pub is_output: bool,
    pub attrs: Vec<Attr>,
}

/// A single IIO device (e.g. `ad9361-phy`, `cf-ad9361-lpc`).
#[derive(Debug, Clone)]
pub struct Device {
    pub id: String,
    pub name: Option<String>,
    pub channels: Vec<Channel>,
    pub attrs: Vec<Attr>,
}

/// Protocol version reported by the remote `iiod`.
#[derive(Debug, Clone)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub git_tag: String,
}

/// A connection to a remote `iiod` daemon.
pub struct Context {
    transport: Box<dyn Transport>,
    pub version: Version,
    pub devices: Vec<Device>,
    pub xml: String,
}

impl Context {
    /// Connect to an `iiod` instance, perform the VERSION handshake, and
    /// enumerate all devices via PRINT.
    pub fn connect(addr: &str) -> Result<Self> {
        Self::connect_with_timeout(addr, Duration::from_secs(3))
    }

    /// Like [`connect`](Self::connect) but with a custom TCP timeout.
    pub fn connect_with_timeout(addr: &str, timeout: Duration) -> Result<Self> {
        let transport = TcpTransport::connect(addr, timeout)?;
        Self::from_transport(Box::new(transport))
    }

    /// Build a context from any [`Transport`] implementation.
    /// Useful for testing with mock transports.
    pub fn from_transport(mut transport: Box<dyn Transport>) -> Result<Self> {
        let (major, minor, git_tag) = protocol::version(transport.as_mut())?;
        let version = Version {
            major,
            minor,
            git_tag,
        };

        let xml = protocol::print(transport.as_mut())?;
        let devices = parse_xml(&xml)?;

        Ok(Self {
            transport,
            version,
            devices,
            xml,
        })
    }

    /// Get a reference to a device by name (e.g. `"ad9361-phy"`).
    pub fn find_device(&self, name: &str) -> Option<&Device> {
        self.devices
            .iter()
            .find(|d| d.name.as_deref() == Some(name) || d.id == name)
    }

    /// Get a mutable reference to the underlying transport for sending
    /// further commands.
    pub fn transport_mut(&mut self) -> &mut dyn Transport {
        self.transport.as_mut()
    }
}

/// Parse the XML output of the PRINT command into a list of devices.
fn parse_xml(xml: &str) -> Result<Vec<Device>> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);

    let mut devices = Vec::new();
    let mut current_device: Option<Device> = None;
    let mut current_channel: Option<Channel> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let local_name = e.local_name();
                let tag = std::str::from_utf8(local_name.as_ref()).unwrap_or("");

                match tag {
                    "device" => {
                        let mut dev = Device {
                            id: String::new(),
                            name: None,
                            channels: Vec::new(),
                            attrs: Vec::new(),
                        };
                        for attr in e.attributes().flatten() {
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            let val = String::from_utf8_lossy(&attr.value).to_string();
                            match key {
                                "id" => dev.id = val,
                                "name" => dev.name = Some(val),
                                _ => {}
                            }
                        }
                        current_device = Some(dev);
                    }
                    "channel" => {
                        let mut ch = Channel {
                            id: String::new(),
                            name: None,
                            is_output: false,
                            attrs: Vec::new(),
                        };
                        for attr in e.attributes().flatten() {
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            let val = String::from_utf8_lossy(&attr.value).to_string();
                            match key {
                                "id" => ch.id = val,
                                "name" => ch.name = Some(val),
                                "type" => ch.is_output = val == "output",
                                _ => {}
                            }
                        }
                        current_channel = Some(ch);
                    }
                    "attribute" => {
                        let mut a = Attr {
                            name: String::new(),
                            filename: String::new(),
                        };
                        for attr in e.attributes().flatten() {
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            let val = String::from_utf8_lossy(&attr.value).to_string();
                            match key {
                                "name" => a.name = val,
                                "filename" => a.filename = val,
                                _ => {}
                            }
                        }
                        if let Some(ref mut ch) = current_channel {
                            ch.attrs.push(a);
                        } else if let Some(ref mut dev) = current_device {
                            dev.attrs.push(a);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let local_name = e.local_name();
                let tag = std::str::from_utf8(local_name.as_ref()).unwrap_or("");
                match tag {
                    "device" => {
                        if let Some(dev) = current_device.take() {
                            devices.push(dev);
                        }
                    }
                    "channel" => {
                        if let Some(ch) = current_channel.take() {
                            if let Some(ref mut dev) = current_device {
                                dev.channels.push(ch);
                            }
                        }
                    }
                    "attribute" => {}
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(Error::Xml(format!("XML parse error: {e}"))),
            _ => {}
        }
    }

    Ok(devices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_xml_basic() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE context [
]>
<context name="network" description="no description">
    <device id="iio:device0" name="ad9361-phy">
        <channel id="voltage0" type="input" name="in_voltage0">
            <attribute name="rf_bandwidth" filename="in_voltage_rf_bandwidth" />
            <attribute name="sampling_frequency" filename="in_voltage_sampling_frequency" />
        </channel>
        <channel id="voltage0" type="output" name="out_voltage0">
            <attribute name="rf_bandwidth" filename="out_voltage_rf_bandwidth" />
        </channel>
        <attribute name="ensm_mode" filename="ensm_mode" />
    </device>
    <device id="iio:device1" name="cf-ad9361-lpc">
        <channel id="voltage0" type="input">
            <attribute name="raw" filename="in_voltage0_raw" />
        </channel>
    </device>
</context>"#;

        let devices = parse_xml(xml).unwrap();
        assert_eq!(devices.len(), 2);

        let phy = &devices[0];
        assert_eq!(phy.id, "iio:device0");
        assert_eq!(phy.name.as_deref(), Some("ad9361-phy"));
        assert_eq!(phy.channels.len(), 2);
        assert_eq!(phy.attrs.len(), 1);
        assert_eq!(phy.attrs[0].name, "ensm_mode");

        let ch0 = &phy.channels[0];
        assert_eq!(ch0.id, "voltage0");
        assert!(!ch0.is_output);
        assert_eq!(ch0.attrs.len(), 2);

        let ch1 = &phy.channels[1];
        assert!(ch1.is_output);

        let lpc = &devices[1];
        assert_eq!(lpc.name.as_deref(), Some("cf-ad9361-lpc"));
        assert_eq!(lpc.channels.len(), 1);
    }
}
