use pci_info::pci_enums::PciDeviceClass;
use pci_info::pci_enums::PciDeviceClass::{Bridge, DisplayController};
use pci_info::{PciDeviceEnumerationError, PciInfo, PciInfoError, PciInfoPropertyError};
use std::collections::{HashMap, HashSet};
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

pub fn get_gpu_pci() -> Result<HashSet<PciLocation>, PciError> {
    let info = PciInfo::enumerate_pci()?;
    // List of GPU devices
    let mut gpu_devices = HashSet::new();
    // Map of all PCI devices with their classes, for IOMMU groups processing
    let mut pci_devices = HashMap::new();

    for device in info {
        let device = device?;
        let device_class = process_property_result(device.device_class())?;
        let device_location = process_property_result(device.location())?;
        if device_class == DisplayController {
            gpu_devices.insert(device_location);
        }
        pci_devices.insert(device_location, device_class);
    }

    match get_iommu_groups() {
        Ok(iommu_groups) => {
            // Find all devices that are in the same IOMMU group as the GPU devices
            let result = iommu_groups
                .into_iter()
                .filter(|(_, devices)| {
                    // Find if this IOMMU groups contains a GPU device
                    gpu_devices
                        .iter()
                        .any(|gpu_device| devices.contains(gpu_device))
                })
                .flat_map(|(_, devices)| devices)
                // We want to filter non-endpoint devies
                .filter(|device| pci_devices.get(device).map_or(false, is_endpoint_device))
                .collect::<HashSet<_>>();
            Ok(result)
        }
        Err(err) => {
            tracing::warn!(
                "Couldn't get IOMMU groups: {err}. Ignoring groups, provide list of PCI nevertheless: {gpu_devices:?}",
            );
            Ok(gpu_devices)
        }
    }
}

// AFAIK the bridge devices are the only non-endpoint devices
// May require to update this function if there are other non-endpoint devices
fn is_endpoint_device(device_class: &PciDeviceClass) -> bool {
    *device_class != Bridge
}

fn process_property_result<T>(result: Result<T, &PciInfoPropertyError>) -> Result<T, PciError> {
    match result {
        Ok(device) => Ok(device),
        Err(PciInfoPropertyError::Unsupported) => Err(PciError::UnsupportedProperty),
        Err(PciInfoPropertyError::Error(err)) => Err(PciError::PciInfoPropertyError(err.clone())),
    }
}

type IommuGroup = HashMap<String, HashSet<PciLocation>>;

fn get_iommu_groups() -> Result<IommuGroup, std::io::Error> {
    const IOMMU_GROUP_PATH: &str = "/sys/kernel/iommu_groups";
    let mut devices_by_group = HashMap::new();

    for iommu_group_result in std::fs::read_dir(IOMMU_GROUP_PATH)? {
        if let Ok(iommu_group) = iommu_group_result {
            let group_name = iommu_group.file_name().to_string_lossy().to_string();

            let path = iommu_group.path().join("devices");

            let group = std::fs::read_dir(path)?
                .flatten()
                .filter_map(|device| {
                    parse_pci_location(device.file_name().to_string_lossy().to_string())
                })
                .collect::<HashSet<_>>();

            devices_by_group.insert(group_name, group);
        } else {
            tracing::warn!("cannot get an IOMMU group: {:?}", iommu_group_result);
        }
    }

    Ok(devices_by_group)
}

// Location format:
// <segment>:<bus>:<device>.<function>: 0000:00:00.0
fn parse_pci_location(location: String) -> Option<PciLocation> {
    let parts: Vec<&str> = location.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    // the segment part is always 0
    let bus = u8::from_str_radix(parts[1], 16).ok()?;
    let device = u8::from_str_radix(parts[2].split('.').next()?, 16).ok()?;
    let function = u8::from_str_radix(parts[2].split('.').last()?, 16).ok()?;

    PciLocation::with_bdf(bus, device, function).ok()
}
