use std::path::PathBuf;

use nonempty::NonEmpty;
use thiserror::Error;
use virt::connect::Connect;
use virt::domain::Domain;
use virt::sys::VIR_DOMAIN_DEFINE_VALIDATE;

#[derive(Debug)]
pub struct CreateVMParams {
    name: String,
    image: PathBuf,
    cpus: NonEmpty<u32>,
}

#[derive(Error, Debug)]
pub enum VMUtilsError {
    #[error("Failed to connect to the hypervisor")]
    FailedToConnect {
        #[source]
        err: virt::error::Error,
    },
    #[error("Failed to create VM")]
    FailedToCreateVM {
        #[source]
        err: virt::error::Error,
    },
    #[error("Could not find VM with name {name}")]
    VmNotFound {
        name: String,
        #[source]
        err: virt::error::Error,
    },
    #[error("Failed to shutdown VM")]
    FailedToShutdownVM {
        #[source]
        err: virt::error::Error,
    },
    #[error("Failed to get id for VM with name {name}")]
    FailedToGetVMId { name: String },
}

pub fn create_vm(uri: &str, params: CreateVMParams) -> Result<(), VMUtilsError> {
    let conn = Connect::open(uri).map_err(|err| VMUtilsError::FailedToConnect { err })?;
    let domain = Domain::lookup_by_name(&conn, params.name.as_str()).ok();

    match domain {
        None => {
            tracing::info!(target: "vm-utils","Domain with name {} doesn't exists. Creating", params.name);
            let xml = prepare_xml(&params);
            Domain::define_xml_flags(&conn, xml.as_str(), VIR_DOMAIN_DEFINE_VALIDATE)
                .map_err(|err| VMUtilsError::FailedToCreateVM { err })?;
        }
        Some(_) => {
            tracing::info!(target: "vm-utils","Domain with name {} already exists. Skipping", params.name);
        }
    };
    Ok(())
}

pub fn start_vm(uri: &str, name: String) -> Result<u32, VMUtilsError> {
    tracing::info!(target: "vm-utils","Starting VM with name {name}");
    let conn = Connect::open(uri).map_err(|err| VMUtilsError::FailedToConnect { err })?;
    let domain =
        Domain::lookup_by_name(&conn, name.as_str()).map_err(|err| VMUtilsError::VmNotFound {
            name: name.clone(),
            err,
        })?;
    domain
        .create()
        .map_err(|err| VMUtilsError::FailedToCreateVM { err })?;

    let id = domain
        .get_id()
        .ok_or(VMUtilsError::FailedToGetVMId { name })?;

    Ok(id)
}

pub fn stop_vm(uri: &str, name: String) -> Result<(), VMUtilsError> {
    tracing::info!(target: "vm-utils","Stopping VM with name {name}");
    let conn = Connect::open(uri).map_err(|err| VMUtilsError::FailedToConnect { err })?;
    let domain = Domain::lookup_by_name(&conn, name.as_str())
        .map_err(|err| VMUtilsError::VmNotFound { name, err })?;
    domain
        .shutdown()
        .map_err(|err| VMUtilsError::FailedToShutdownVM { err })?;

    Ok(())
}

fn prepare_xml(params: &CreateVMParams) -> String {
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
        params.image.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use nonempty::nonempty;
    use std::fs;
    const DEFAULT_URI: &str = "test:///default";

    fn list_defined() -> Result<Vec<String>, VMUtilsError> {
        let conn =
            Connect::open(DEFAULT_URI).map_err(|err| VMUtilsError::FailedToConnect { err })?;
        Ok(conn.list_defined_domains().unwrap())
    }

    fn list() -> Result<Vec<u32>, VMUtilsError> {
        let conn =
            Connect::open(DEFAULT_URI).map_err(|err| VMUtilsError::FailedToConnect { err })?;
        Ok(conn.list_domains().unwrap())
    }

    #[test]
    fn test_prepare_xml() {
        let xml = prepare_xml(&CreateVMParams {
            name: "test-id".to_string(),
            image: "test-image".into(),
            cpus: nonempty![1, 8],
        });
        assert_eq!(xml, include_str!("../tests/expected_vm_config.xml"))
    }

    #[test]
    fn test_vm_creation() {
        log_utils::enable_logs();

        let image: PathBuf = "./src/alpine-virt-3.20.1-x86_64.iso".into();
        let image = fs::canonicalize(image).unwrap();

        let list_before_create = list().unwrap();
        let list_defined_before_create = list_defined().unwrap();
        assert!(list_defined_before_create.is_empty());

        create_vm(
            DEFAULT_URI,
            CreateVMParams {
                name: "test-id".to_string(),
                image: image.clone(),
                cpus: nonempty![1],
            },
        )
        .unwrap();

        let list_after_create = list().unwrap();
        let list_defined_after_create = list_defined().unwrap();
        assert_eq!(list_defined_after_create, vec!["test-id"]);
        assert_eq!(list_after_create, list_before_create);

        let id = start_vm(DEFAULT_URI, "test-id".to_string()).unwrap();

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
