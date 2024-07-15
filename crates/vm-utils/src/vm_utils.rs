use std::path::PathBuf;

use nonempty::NonEmpty;
use thiserror::Error;
use virt::connect::Connect;
use virt::domain::Domain;

#[derive(Debug)]
pub struct CreateVMParams {
    name: String,
    image: PathBuf,
    cpus: NonEmpty<u32>,
}

#[derive(Error, Debug)]
pub enum CreateVMError {
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

pub fn create_vm(params: CreateVMParams) -> Result<(), CreateVMError> {
    let conn =
        Connect::open("test:///default").map_err(|err| CreateVMError::FailedToConnect { err })?;
    let domain = Domain::lookup_by_name(&conn, params.name.as_str()).ok();

    match domain {
        None => {
            let xml = prepare_xml(&params);
            Domain::define_xml(&conn, xml.as_str())
                .map_err(|err| CreateVMError::FailedToCreateVM { err })?;
        }
        Some(_) => {
            tracing::info!("Domain with name {} already exists", params.name);
        }
    };
    Ok(())
}

pub fn start_vm(name: String) -> Result<u32, CreateVMError> {
    let conn =
        Connect::open("test:///default").map_err(|err| CreateVMError::FailedToConnect { err })?;
    let domain =
        Domain::lookup_by_name(&conn, name.as_str()).map_err(|err| CreateVMError::VmNotFound {
            name: name.clone(),
            err,
        })?;
    domain
        .create()
        .map_err(|err| CreateVMError::FailedToCreateVM { err })?;

    let id = domain
        .get_id()
        .ok_or(CreateVMError::FailedToGetVMId { name })?;

    Ok(id)
}

pub fn stop_vm(name: String) -> Result<(), CreateVMError> {
    let conn =
        Connect::open("test:///default").map_err(|err| CreateVMError::FailedToConnect { err })?;
    let domain = Domain::lookup_by_name(&conn, name.as_str())
        .map_err(|err| CreateVMError::VmNotFound { name, err })?;
    domain
        .shutdown()
        .map_err(|err| CreateVMError::FailedToShutdownVM { err })?;

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

    fn list_defined() -> Result<Vec<String>, CreateVMError> {
        let conn = Connect::open("test:///default")
            .map_err(|err| CreateVMError::FailedToConnect { err })?;
        Ok(conn.list_defined_domains().unwrap())
    }

    fn list() -> Result<Vec<u32>, CreateVMError> {
        let conn = Connect::open("test:///default")
            .map_err(|err| CreateVMError::FailedToConnect { err })?;
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
    fn it_works() {
        let image: PathBuf = "./src/alpine-virt-3.20.1-x86_64.iso".into();
        let image = fs::canonicalize(image).unwrap();

        let list_before_create = list().unwrap();
        let list_defined_before_create = list_defined().unwrap();
        assert!(list_defined_before_create.is_empty());

        create_vm(CreateVMParams {
            name: "test-id".to_string(),
            image,
            cpus: nonempty![1],
        })
        .unwrap();

        let list_after_create = list().unwrap();
        let list_defined_after_create = list_defined().unwrap();
        assert_eq!(list_defined_after_create, vec!["test-id"]);
        assert_eq!(list_after_create, list_before_create);

        let id = start_vm("test-id".to_string()).unwrap();

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
