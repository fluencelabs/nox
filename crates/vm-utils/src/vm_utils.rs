use ccp_shared::types::LogicalCoreId;
use gpu_utils::PciLocation;
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
    cores_num: usize,
    bridge_name: String,
    allow_gpu: bool,
}

impl CreateVMDomainParams {
    pub fn new(
        name: String,
        image: PathBuf,
        cpus: NonEmpty<LogicalCoreId>,
        cores_num: usize,
        bridge_name: String,
        allow_gpu: bool,
    ) -> Self {
        Self {
            name,
            image,
            cpus,
            cores_num,
            bridge_name,
            allow_gpu,
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
    #[error("Failed to get GPU PCI location: {0}")]
    FailedToGetPCI(#[from] gpu_utils::PciError),
}

// The list of states is taken from the libvirt documentation
#[derive(Debug, PartialEq, Eq)]
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

struct AutocloseConnect(Connect);

impl AutocloseConnect {
    fn open(uri: &str) -> Result<Self, VmError> {
        Connect::open(Some(uri))
            .map(|conn| Self(conn))
            .map_err(|err| VmError::FailedToConnect { err })
    }
}

impl std::ops::Deref for AutocloseConnect {
    type Target = Connect;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for AutocloseConnect {
    fn drop(&mut self) {
        match self.0.close() {
            Ok(_) => {}
            Err(err) => {
                tracing::warn!(target: "vm-utils","Failed to close connection properly: {err}");
            }
        }
    }
}

pub fn create_domain(uri: &str, params: &CreateVMDomainParams) -> Result<(), VmError> {
    let conn = AutocloseConnect::open(uri)?;
    let domain = Domain::lookup_by_name(&conn, params.name.as_str()).ok();

    match domain {
        None => {
            tracing::info!(target: "vm-utils","Domain with name {} doesn't exists. Creating", params.name);
            // There's certainly better places to do this, but RN it doesn't really matter
            let gpu_pci_locations = if params.allow_gpu {
                tracing::info!(target: "gpu-utils", "Collecting info about GPU devices...");
                gpu_utils::get_gpu_pci()?.into_iter().collect::<Vec<_>>()
            } else {
                vec![]
            };
            let xml = prepare_xml(uri, params, &gpu_pci_locations);
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
    let conn = AutocloseConnect::open(uri)?;
    let domain = Domain::lookup_by_name(&conn, name).map_err(|err| VmError::VmNotFound {
        name: name.to_string(),
        err,
    })?;

    if let Err(error) = domain.destroy() {
        tracing::warn!(target: "vm-utils","Failed to destroy VM {name}: {error}");
        let status = get_status(&domain).map_err(|err| VmError::FailedToGetInfo {
            name: name.to_string(),
            err,
        })?;
        tracing::warn!(target: "vm-utils","VM {name} in the state: {status}");
        if status != VmStatus::Shutoff {
            return Err(VmError::FailedToRemoveVMDomain {
                err: error,
                name: name.to_string(),
            });
        }
    }

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
    let conn = AutocloseConnect::open(uri)?;
    let domain = Domain::lookup_by_name(&conn, name).map_err(|err| VmError::VmNotFound {
        name: name.to_string(),
        err,
    })?;

    let is_running = domain.is_active().map_err(|err| VmError::FailedToStartVM {
        err,
        name: name.to_string(),
    })?;

    if is_running {
        tracing::info!(target: "vm-utils","VM with name {name} is already running");
    } else {
        domain.create().map_err(|err| VmError::FailedToStartVM {
            err,
            name: name.to_string(),
        })?;
    }

    let id = domain.get_id().ok_or(VmError::FailedToGetVMId {
        name: name.to_string(),
    })?;

    Ok(id)
}

pub fn stop_vm(uri: &str, name: &str) -> Result<(), VmError> {
    tracing::info!(target: "vm-utils","Stopping VM with name {name}");
    let conn = AutocloseConnect::open(uri)?;
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
    let conn = AutocloseConnect::open(uri)?;
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
    let conn = AutocloseConnect::open(uri)?;
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
    let conn = AutocloseConnect::open(uri)?;
    let domain = Domain::lookup_by_name(&conn, name).map_err(|err| VmError::VmNotFound {
        name: name.to_string(),
        err,
    })?;
    let status = get_status(&domain).map_err(|err| VmError::FailedToGetInfo {
        name: name.to_string(),
        err,
    });
    status
}

fn get_status(domain: &Domain) -> Result<VmStatus, virt::error::Error> {
    let info = domain.get_info()?;

    Ok(VmStatus::from_u32(info.state))
}

pub fn pause_vm(uri: &str, name: &str) -> Result<(), VmError> {
    tracing::info!(target: "vm-utils","Pausing VM with name {name}");
    let conn = Connect::open(Some(uri)).map_err(|err| VmError::FailedToConnect { err })?;
    let domain = Domain::lookup_by_name(&conn, name).map_err(|err| VmError::VmNotFound {
        name: name.to_string(),
        err,
    })?;

    if let Err(err) = domain.suspend() {
        tracing::warn!(target: "vm-utils","Failed to suspend VM {name}: {err}");
        let status = get_status(&domain).map_err(|err| VmError::FailedToGetInfo {
            name: name.to_string(),
            err,
        })?;
        tracing::warn!(target: "vm-utils","VM {name} in the state: {status}");
        if status == VmStatus::Running {
            return Err(VmError::FailedToSuspendVM {
                name: name.to_string(),
                err,
            });
        }
    }
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

fn allocate_ram_mib(physical_cores_num: usize) -> usize {
    // 4GB per core
    physical_cores_num * 4 * 1024
}

fn prepare_xml(
    libvirt_uri: &str,
    params: &CreateVMDomainParams,
    gpu_pci_location: &[PciLocation],
) -> String {
    match prepare_xml_from_cli(libvirt_uri, params, gpu_pci_location) {
        Ok(xml) => xml,
        Err(err) => {
            tracing::warn!(target: "vm-utils","Failed to prepare XML using CLI: {err}. Falling back to template");
            prepare_xml_from_template(params, &generate_random_mac().to_string(), gpu_pci_location)
        }
    }
}

#[derive(Debug, Error)]
enum PrepareXmlError {
    #[error("error while executing the command: {0}")]
    IoError(#[from] std::io::Error),
    #[error("error while converting the command output into a string: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("command exited with error code {status}: {stderr:?}")]
    Error {
        status: String,
        stderr: Result<String, std::string::FromUtf8Error>,
    },
}

// Command example:
// virt-install
//   # Where to connect
//   --connect qemu:///system
//   # VM name
//   --name test3
//   # How the cores are pinned
//   --vcpus 2,placement=static
//   --cputune vcpupin0.vcpu=0,vcpupin0.cpuset=1,vcpupin1.vcpu=1,vcpupin1.cpuset=4
//   # Amout of RAM allocated to the VM
//   --ram 3000
//   # Path to a qcow2 disk
//   --disk alpine-virt-3.20.1-x86_64.qcow2
//   # Use the default network, the brige name isn't in use with this thing
//   --network network=default
//   # import GPUs and other devices in the IOMMU group
//   --hostdev 01:00.0,address.type=pci,address.multifunction=on
//   --hostdev 01:00.1,address.type=pci,address.multifunction=on
//    # detect OS, since we don't know which OS will be on a VM
//   --osinfo detect=on,require=off
//   # skip OS insallation process, build VM around prepared disk
//   --import
//   --print-xml
fn prepare_xml_from_cli(
    libvirt_uri: &str,
    params: &CreateVMDomainParams,
    gpu_pci_location: &[PciLocation],
) -> Result<String, PrepareXmlError> {
    let command = cli_command(libvirt_uri, params, gpu_pci_location);

    // Execute the command
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()?;
    if !output.status.success() {
        return Err(PrepareXmlError::Error {
            status: output.status.to_string(),
            stderr: String::from_utf8(output.stderr),
        });
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn cli_command(
    libvirt_uri: &str,
    params: &CreateVMDomainParams,
    gpu_pci_location: &[PciLocation],
) -> String {
    let pin_cores_argument = &params
        .cpus
        .iter()
        .enumerate()
        .map(|(index, logical_id)| {
            format!("vcpupin{index}.vcpu={index},vcpupin{index}.cpuset={logical_id}")
        })
        .collect::<Vec<_>>()
        .join(",");

    let gpus_arument = gpu_pci_location
        .iter()
        .map(|location| format!("--hostdev {location},address.type=pci,address.multifunction=on",))
        .collect::<Vec<_>>()
        .join(" ");

    vec![
        "virt-install",
        "--connect",
        libvirt_uri,
        "--name",
        &params.name,
        "--vcpus",
        &format!("{},placement=static", params.cpus.len()),
        "--cputune",
        pin_cores_argument,
        "--memory",
        &format!("{}", allocate_ram_mib(params.cores_num)),
        "--disk",
        params.image.to_str().unwrap(),
        "--network",
        "network=default",
        &gpus_arument,
        "--osinfo",
        "detect=on,require=off",
        "--import",
        "--print-xml",
    ]
    .join(" ")
}

fn prepare_xml_from_template(
    params: &CreateVMDomainParams,
    mac_address: &str,
    gpu_pci_location: &[PciLocation],
) -> String {
    fn prepare_pci_config(location: &PciLocation) -> String {
        format!(
            r#"
        <hostdev mode="subsystem" type="pci" managed="yes">
            <source>
                <address domain="{domain}" bus="{bus}" slot="{slot}" function="{function}"/>
            </source>
        </hostdev>"#,
            domain = location.segment(),
            bus = location.bus(),
            slot = location.device(),
            function = location.function(),
        )
    }

    let mut mapping = String::new();
    for (index, logical_id) in params.cpus.iter().enumerate() {
        if index > 0 {
            mapping.push_str("\n        ");
        }
        mapping.push_str(format!("<vcpupin vcpu='{index}' cpuset='{logical_id}'/>").as_str());
    }
    let memory_in_mib = allocate_ram_mib(params.cores_num);
    let gpu_pci_configuration = gpu_pci_location
        .iter()
        .map(prepare_pci_config)
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        include_str!("template.xml"),
        params.name,
        memory_in_mib,
        params.cpus.len(),
        mapping,
        params.image.display(),
        mac_address,
        params.bridge_name,
        gpu_pci_configuration,
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
    fn test_cli_command() {
        let params = CreateVMDomainParams {
            name: "test-id".to_string(),
            image: "test-image".into(),
            cpus: nonempty![2.into(), 8.into()],
            cores_num: 2,
            bridge_name: "br422442".to_string(),
            allow_gpu: true,
        };
        let gpu = &[
            PciLocation::with_bdf(1, 0, 0).unwrap(),
            PciLocation::with_bdf(2, 0, 0).unwrap(),
        ];
        let result = cli_command("qemu:///system", &params, gpu);
        let expected = "virt-install --connect qemu:///system --name test-id --vcpus 2,placement=static --cputune vcpupin0.vcpu=0,vcpupin0.cpuset=2,vcpupin1.vcpu=1,vcpupin1.cpuset=8 --memory 8192 --disk test-image --network network=default --hostdev 0000:01:00.0,address.type=pci,address.multifunction=on --hostdev 0000:02:00.0,address.type=pci,address.multifunction=on --osinfo detect=on,require=off --import --print-xml";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_cli_command_without_gpu() {
        let params = CreateVMDomainParams {
            name: "test-id".to_string(),
            image: "test-image".into(),
            cpus: nonempty![2.into(), 8.into()],
            cores_num: 2,
            bridge_name: "br422442".to_string(),
            allow_gpu: true,
        };
        let result = cli_command("qemu:///system", &params, &[]);
        // This string contains a space where a GPU argument should be just to avoid a lot of string manipulation
        let expected = "virt-install --connect qemu:///system --name test-id --vcpus 2,placement=static --cputune vcpupin0.vcpu=0,vcpupin0.cpuset=2,vcpupin1.vcpu=1,vcpupin1.cpuset=8 --memory 8192 --disk test-image --network network=default  --osinfo detect=on,require=off --import --print-xml";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_prepare_xml() {
        let xml = prepare_xml_from_template(
            &CreateVMDomainParams {
                name: "test-id".to_string(),
                image: "test-image".into(),
                cpus: nonempty![1.into(), 8.into()],
                cores_num: 2,
                bridge_name: "br422442".to_string(),
                allow_gpu: true,
            },
            "52:54:00:1e:af:64",
            &[
                PciLocation::with_bdf(1, 0, 0).unwrap(),
                PciLocation::with_bdf(2, 0, 0).unwrap(),
            ],
        );

        // trim excessive whitespaces
        let xml = xml
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        let expected = include_str!("../tests/expected_vm_config.xml");
        let expected = expected
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(xml, expected);
    }

    #[test]
    fn test_prepare_xml_without_gpu() {
        let xml = prepare_xml_from_template(
            &CreateVMDomainParams {
                name: "test-id".to_string(),
                image: "test-image".into(),
                cpus: nonempty![1.into(), 8.into()],
                cores_num: 2,
                bridge_name: "br422442".to_string(),
                allow_gpu: false,
            },
            "52:54:00:1e:af:64",
            &[],
        );

        // trim excessive whitespaces
        let xml = xml
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        let expected = include_str!("../tests/expected_vm_config_without_gpu.xml");
        let expected = expected
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(xml, expected);
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
                cores_num: 1,
                bridge_name: "br422442".to_string(),
                allow_gpu: false,
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
