#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputDevice {
    pub id: Option<String>,
    pub name: String,
}

impl Default for OutputDevice {
    fn default() -> Self {
        Self {
            id: None,
            name: "Default output · follows Windows".into(),
        }
    }
}
impl std::fmt::Display for OutputDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

pub fn output_devices() -> Result<Vec<OutputDevice>, String> {
    // COM apartment ownership stays on this short-lived enumeration thread.
    std::thread::spawn(enumerate)
        .join()
        .map_err(|_| "Output-device enumeration failed".to_string())?
}

#[cfg(not(windows))]
fn enumerate() -> Result<Vec<OutputDevice>, String> {
    Ok(vec![OutputDevice::default()])
}

#[cfg(windows)]
fn enumerate() -> Result<Vec<OutputDevice>, String> {
    wasapi::initialize_mta().ok().map_err(|e| e.to_string())?;
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            wasapi::deinitialize();
        }
    }
    let _guard = Guard;
    let enumerator = wasapi::DeviceEnumerator::new().map_err(|e| e.to_string())?;
    let collection = enumerator
        .get_device_collection(wasapi::Direction::Render)
        .map_err(|e| e.to_string())?;
    let mut devices = vec![OutputDevice::default()];
    for device in &collection {
        let device = device.map_err(|e| e.to_string())?;
        devices.push(OutputDevice {
            id: Some(device.get_id().map_err(|e| e.to_string())?),
            name: device.get_friendlyname().map_err(|e| e.to_string())?,
        });
    }
    Ok(devices)
}
