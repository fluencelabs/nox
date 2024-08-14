use ccp_shared::types::LogicalCoreId;
use mac_address::MacAddress;
use nonempty::NonEmpty;
use rand::Rng;
use std::fmt::Display;
use std::path::PathBuf;
use thiserror::Error;
use virt::connect::Connect;
use virt::domain::Domain;
use virt::sys::{VIR_DOMAIN_DEFINE_VALIDATE, VIR_DOMAIN_REBOOT_DEFAULT};

const MAC_PREFIX: [u8; 3] = [0x52, 0x54, 0x00];

#[derive(Debug)]
pub struct CreateVMDomainParams {
    name: String,
    image: PathBuf,
    cpus: NonEmpty<LogicalCoreId>,
    bridge_name: String,
}

impl CreateVMDomainParams {
    pub fn new(
        name: String,
        image: PathBuf,
        cpus: NonEmpty<LogicalCoreId>,
        bridge_name: String,
    ) -> Self {
        Self {
            name,
            image,
            cpus,
            bridge_name,
        }
    }
}

#[derive(Error, Debug)]
pub enum VmError {
    #[error("Failed to connect to the hypervisor: {err}")]
    FailedToConnect {
        #[source]
        err: virt::error::Error,
    },
    #[error("Failed to create VM domain: {err}")]
    FailedToCreateVMDomain {
        #[source]
        err: virt::error::Error,
    },
    #[error("Failed to remove VM domain {name}: {err}")]
    FailedToRemoveVMDomain {
        #[source]
        err: virt::error::Error,
        name: String,
    },
    #[error("Failed to run VM {name}: {err}")]
    FailedToStartVM {
        #[source]
        err: virt::error::Error,
        name: String,
    },
    #[error("Could not find VM with name {name}: {err}")]
    VmNotFound {
        name: String,
        #[source]
        err: virt::error::Error,
    },
    #[error("Failed to shutdown VM {name}: {err}")]
    FailedToStopVM {
        name: String,
        #[source]
        err: virt::error::Error,
    },
    #[error("Failed to get id for VM with name {name}")]
    FailedToGetVMId { name: String },
    #[error("Failed to reboot {name}: {err}")]
    FailedToRebootVM {
        err: virt::error::Error,
        name: String,
    },
    #[error("Failed to reset {name}: {err}")]
    FailedToResetVM {
        err: virt::error::Error,
        name: String,
    },
    #[error("Failed to get info for {name}: {err}")]
    FailedToGetInfo {
        err: virt::error::Error,
        name: String,
    },
    #[error("Failed to suspend VM {name}: {err}")]
    FailedToSuspendVM {
        name: String,
        #[source]
        err: virt::error::Error,
    },

    #[error("Failed to resume VM {name}: {err}")]
    FailedToResumeVM {
        name: String,
        #[source]
        err: virt::error::Error,
    },
}

// The list of states is taken from the libvirt documentation
#[derive(Debug)]
pub enum VmStatus {
    NoState,
    Running,
    Blocked,
    Paused,
    Shutdown,
    Shutoff,
    Crashed,
    PMSuspended,
    UnknownState(u32),
}

impl Display for VmStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmStatus::NoState => write!(f, "No state"),
            VmStatus::Running => write!(f, "Running"),
            VmStatus::Blocked => write!(f, "Blocked"),
            VmStatus::Paused => write!(f, "Paused"),
            VmStatus::Shutdown => write!(f, "Shutdown"),
            VmStatus::Shutoff => write!(f, "Shutoff"),
            VmStatus::Crashed => write!(f, "Crashed"),
            VmStatus::PMSuspended => write!(f, "PMSuspended"),
            VmStatus::UnknownState(value) => write!(f, "Unknown state ({})", value),
        }
    }
}

impl VmStatus {
    pub fn from_u32(value: u32) -> Self {
        match value {
            virt::sys::VIR_DOMAIN_NOSTATE => VmStatus::NoState,
            virt::sys::VIR_DOMAIN_RUNNING => VmStatus::Running,
            virt::sys::VIR_DOMAIN_BLOCKED => VmStatus::Blocked,
            virt::sys::VIR_DOMAIN_PAUSED => VmStatus::Paused,
            virt::sys::VIR_DOMAIN_SHUTDOWN => VmStatus::Shutdown,
            virt::sys::VIR_DOMAIN_SHUTOFF => VmStatus::Shutoff,
            virt::sys::VIR_DOMAIN_CRASHED => VmStatus::Crashed,
            virt::sys::VIR_DOMAIN_PMSUSPENDED => VmStatus::PMSuspended,
            _ => VmStatus::UnknownState(value),
        }
    }
}

pub fn create_domain(uri: &str, params: &CreateVMDomainParams) -> Result<(), VmError> {
    let conn = Connect::open(Some(uri)).map_err(|err| VmError::FailedToConnect { err })?;
    let domain = Domain::lookup_by_name(&conn, params.name.as_str()).ok();

    match domain {
        None => {
            tracing::info!(target: "vm-utils","Domain with name {} doesn't exists. Creating", params.name);
            let mac = generate_random_mac();
            let xml = prepare_xml(&params, mac.to_string().as_str());
            Domain::define_xml_flags(&conn, xml.as_str(), VIR_DOMAIN_DEFINE_VALIDATE)
                .map_err(|err| VmError::FailedToCreateVMDomain { err })?;
        }
        Some(_) => {
            tracing::info!(target: "vm-utils","Domain with name {} already exists. Skipping", params.name);
        }
    };
    Ok(())
}

pub fn remove_domain(uri: &str, name: &str) -> Result<(), VmError> {
    tracing::info!(target: "vm-utils","Removing domain with name {}", name);
    let conn = Connect::open(Some(uri)).map_err(|err| VmError::FailedToConnect { err })?;
    let domain = Domain::lookup_by_name(&conn, name).map_err(|err| VmError::VmNotFound {
        name: name.to_string(),
        err,
    })?;

    domain
        .destroy()
        .map_err(|err| VmError::FailedToRemoveVMDomain {
            err,
            name: name.to_string(),
        })?;

    domain
        .undefine()
        .map_err(|err| VmError::FailedToRemoveVMDomain {
            err,
            name: name.to_string(),
        })?;

    Ok(())
}
pub fn start_vm(uri: &str, name: &str) -> Result<u32, VmError> {
    tracing::info!(target: "vm-utils","Starting VM with name {name}");
    let conn = Connect::open(Some(uri)).map_err(|err| VmError::FailedToConnect { err })?;
    let domain = Domain::lookup_by_name(&conn, name).map_err(|err| VmError::VmNotFound {
        name: name.to_string(),
        err,
    })?;
    domain.create().map_err(|err| VmError::FailedToStartVM {
        err,
        name: name.to_string(),
    })?;

    let id = domain.get_id().ok_or(VmError::FailedToGetVMId {
        name: name.to_string(),
    })?;

    Ok(id)
}

pub fn stop_vm(uri: &str, name: &str) -> Result<(), VmError> {
    tracing::info!(target: "vm-utils","Stopping VM with name {name}");
    let conn = Connect::open(Some(uri)).map_err(|err| VmError::FailedToConnect { err })?;
    let domain = Domain::lookup_by_name(&conn, name).map_err(|err| VmError::VmNotFound {
        name: name.to_string(),
        err,
    })?;
    domain.shutdown().map_err(|err| VmError::FailedToStopVM {
        name: name.to_string(),
        err,
    })?;

    Ok(())
}

pub fn reboot_vm(uri: &str, name: &str) -> Result<(), VmError> {
    tracing::info!(target: "vm-utils","Rebooting VM with name {name}");
    let conn = Connect::open(Some(uri)).map_err(|err| VmError::FailedToConnect { err })?;
    let domain = Domain::lookup_by_name(&conn, name).map_err(|err| VmError::VmNotFound {
        name: name.to_string(),
        err,
    })?;
    domain
        .reboot(VIR_DOMAIN_REBOOT_DEFAULT)
        .map_err(|err| VmError::FailedToRebootVM {
            err,
            name: name.to_string(),
        })?;
    Ok(())
}

pub fn reset_vm(uri: &str, name: &str) -> Result<(), VmError> {
    tracing::info!(target: "vm-utils","Resetting VM with name {name}");
    let conn = Connect::open(Some(uri)).map_err(|err| VmError::FailedToConnect { err })?;
    let domain = Domain::lookup_by_name(&conn, name).map_err(|err| VmError::VmNotFound {
        name: name.to_string(),
        err,
    })?;
    domain.reset().map_err(|err| VmError::FailedToResetVM {
        name: name.to_string(),
        err,
    })?;
    Ok(())
}

pub fn status_vm(uri: &str, name: &str) -> Result<VmStatus, VmError> {
    tracing::info!(target: "vm-utils","Getting info for VM with name {name}");
    let conn = Connect::open(Some(uri)).map_err(|err| VmError::FailedToConnect { err })?;
    let domain = Domain::lookup_by_name(&conn, name).map_err(|err| VmError::VmNotFound {
        name: name.to_string(),
        err,
    })?;
    let info = domain.get_info().map_err(|err| VmError::FailedToGetInfo {
        name: name.to_string(),
        err,
    })?;

    Ok(VmStatus::from_u32(info.state))
}

pub fn pause_vm(uri: &str, name: &str) -> Result<(), VmError> {
    tracing::info!(target: "vm-utils","Pausing VM with name {name}");
    let conn = Connect::open(Some(uri)).map_err(|err| VmError::FailedToConnect { err })?;
    let domain = Domain::lookup_by_name(&conn, name).map_err(|err| VmError::VmNotFound {
        name: name.to_string(),
        err,
    })?;
    domain.suspend().map_err(|err| VmError::FailedToSuspendVM {
        name: name.to_string(),
        err,
    })?;
    Ok(())
}

pub fn resume_vm(uri: &str, name: &str) -> Result<(), VmError> {
    tracing::info!(target: "vm-utils","Resuming VM with name {name}");
    let conn = Connect::open(Some(uri)).map_err(|err| VmError::FailedToConnect { err })?;
    let domain = Domain::lookup_by_name(&conn, name).map_err(|err| VmError::VmNotFound {
        name: name.to_string(),
        err,
    })?;
    domain.resume().map_err(|err| VmError::FailedToResumeVM {
        name: name.to_string(),
        err,
    })?;
    Ok(())
}

fn generate_random_mac() -> MacAddress {
    let mut rng = rand::thread_rng();
    let mut result = [0u8; 6];
    result[..3].copy_from_slice(&MAC_PREFIX);
    rng.fill(&mut result[3..]);
    MacAddress::from(result)
}

fn prepare_xml(params: &CreateVMDomainParams, mac_address: &str) -> String {
    let mut mapping = String::new();
    for (index, logical_id) in params.cpus.iter().enumerate() {
        if index > 0 {
            mapping.push_str("\n        ");
        }
        mapping.push_str(format!("<vcpupin vcpu='{index}' cpuset='{logical_id}'/>").as_str());
    }
    let memory_in_kb = params.cpus.len() * 4 * 1024 * 1024; // 4Gbs per core
    format!(
        include_str!("template.xml"),
        params.name,
        memory_in_kb,
        params.cpus.len(),
        mapping,
        params.image.display(),
        mac_address,
        params.bridge_name,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use nonempty::nonempty;
    use std::fs;
    const DEFAULT_URI: &str = "test:///default";

    fn list_defined() -> Result<Vec<String>, VmError> {
        let conn =
            Connect::open(Some(DEFAULT_URI)).map_err(|err| VmError::FailedToConnect { err })?;
        Ok(conn.list_defined_domains().unwrap())
    }

    fn list() -> Result<Vec<u32>, VmError> {
        let conn =
            Connect::open(Some(DEFAULT_URI)).map_err(|err| VmError::FailedToConnect { err })?;
        Ok(conn.list_domains().unwrap())
    }

    #[test]
    fn test_prepare_xml() {
        let xml = prepare_xml(
            &CreateVMDomainParams {
                name: "test-id".to_string(),
                image: "test-image".into(),
                cpus: nonempty![1.into(), 8.into()],
                bridge_name: "br422442".to_string(),
            },
            "52:54:00:1e:af:64",
        );
        assert_eq!(xml, include_str!("../tests/expected_vm_config.xml"))
    }

    #[test]
    fn test_vm_creation() {
        log_utils::enable_logs();

        let image: PathBuf = "./tests/alpine-virt-3.20.1-x86_64.qcow2".into();
        let image = fs::canonicalize(image).unwrap();

        let list_before_create = list().unwrap();
        let list_defined_before_create = list_defined().unwrap();
        assert!(list_defined_before_create.is_empty());

        create_domain(
            DEFAULT_URI,
            &CreateVMDomainParams {
                name: "test-id".to_string(),
                image: image.clone(),
                cpus: nonempty![1.into()],
                bridge_name: "br422442".to_string(),
            },
        )
        .unwrap();

        let list_after_create = list().unwrap();
        let list_defined_after_create = list_defined().unwrap();
        assert_eq!(list_defined_after_create, vec!["test-id"]);
        assert_eq!(list_after_create, list_before_create);

        let id = start_vm(DEFAULT_URI, "test-id").unwrap();

        let mut list_after_start = list().unwrap();
        let list_defined_after_start = list_defined().unwrap();
        let mut expected_list_after_start = Vec::new();
        expected_list_after_start.push(id);
        expected_list_after_start.extend(&list_before_create);

        expected_list_after_start.sort();
        list_after_start.sort();

        assert!(list_defined_after_start.is_empty());
        assert_eq!(list_after_start, expected_list_after_start);
    }
}
