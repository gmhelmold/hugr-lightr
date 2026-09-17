use super::{InstallFailure, InstallStage, InstallVisibility};
use lightr_core::LightrError;
use std::{error::Error, io, path::Path};

#[test]
fn legacy_error_retains_downcastable_phase_and_original_os_cause() {
    let error = InstallFailure::new(InstallStage::SyncDirectory, InstallVisibility::InstalledUnconfirmed, Path::new("objects-ready/aa/bb"), io::Error::from_raw_os_error(28));
    let LightrError::Io(outer) = error.into_legacy() else { panic!("changed frozen variant") };
    let payload = outer.get_ref().unwrap().downcast_ref::<InstallFailure>().unwrap();
    assert_eq!(payload.stage(), InstallStage::SyncDirectory);
    assert_eq!(payload.visibility(), InstallVisibility::InstalledUnconfirmed);
    assert_eq!(payload.path(), Path::new("objects-ready/aa/bb"));
    assert_eq!(payload.io_error().raw_os_error(), Some(28));
    assert_eq!(payload.source().unwrap().downcast_ref::<io::Error>().unwrap().raw_os_error(), Some(28));
    assert!(outer.to_string().contains("InstalledUnconfirmed"));
}
