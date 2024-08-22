use pci_info::pci_enums::PciDeviceClass::DisplayController;
use pci_info::{PciDeviceEnumerationError, PciInfo, PciInfoError, PciInfoPropertyError};
use thiserror::Error;

pub use pci_info::PciLocation;

#[derive(Debug, Error)]
pub enum PciError {
    #[error("can't get list of devices: {0}")]
    PciInfoError(#[from] PciInfoError),
    #[error("can't get info for a device: {0}")]
    PciInfoEnumerationError(#[from] PciDeviceEnumerationError),
    #[error("can't get properties for a device: {0}")]
    PciInfoPropertyError(Box<PciInfoError>),
    #[error("required property is not support for a device")]
    UnsupportedProperty,
}

pub fn get_gpu_pci() -> Result<Vec<PciLocation>, PciError> {
    let info = PciInfo::enumerate_pci()?;
    let mut result = Vec::new();
    for device in info {
        let device = device?;
        let device_class = process_property_result(device.device_class())?;
        if device_class == DisplayController {
            result.push(process_property_result(device.location())?);
        }
    }
    Ok(result)
}

fn process_property_result<T>(result: Result<T, &PciInfoPropertyError>) -> Result<T, PciError> {
    match result {
        Ok(device) => Ok(device),
        Err(PciInfoPropertyError::Unsupported) => Err(PciError::UnsupportedProperty),
        Err(PciInfoPropertyError::Error(err)) => Err(PciError::PciInfoPropertyError(err.clone())),
    }
}
