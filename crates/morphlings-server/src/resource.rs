use std::{fs, path::PathBuf};

use snafu::{ResultExt, whatever};

use crate::{FailedToReadFileSnafu, ServerResult};

#[derive(Debug)]
pub(crate) struct Resource {
    file_path: PathBuf,
}

pub(crate) fn scan_resource(path: PathBuf) -> ServerResult<Vec<Resource>> {
    if !path.exists() {
        whatever!("resrouce not exists");
    }

    collect_files(&path)
}

fn collect_files(path: &PathBuf) -> ServerResult<Vec<Resource>> {
    let mut all_resources = vec![];
    for entry in fs::read_dir(path).context(FailedToReadFileSnafu {
        file_type: "resource dir",
        path: path.to_string_lossy(),
    })? {
        let resource = entry.context(FailedToReadFileSnafu {
            file_type: "resource file",
            path: format!("in {}", path.to_string_lossy()),
        })?;
        all_resources.push(Resource {
            file_path: resource.path(),
        })
    }

    Ok(all_resources)
}
