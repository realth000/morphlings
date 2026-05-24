use std::{fs, path::PathBuf};

use morphlings_apis::Resource;
use snafu::{ResultExt, whatever};

use crate::{FailedToReadFileSnafu, ServerResult};

/// All accepted file extenstions.
///
/// Use these extension suffix to filter files we want to collect.
///
/// Note that this is a temporary solution for file type checking, which should
/// be deprecated or disabled in the future.
const ACCEPTED_EXTENSION: [&'static str; 1] = [".mp3"];

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

        if resource
            .metadata()
            .context(FailedToReadFileSnafu {
                file_type: "resource file",
                path: format!("in {} metadata", path.to_string_lossy()),
            })?
            .is_dir()
        {
            // Directory.
            all_resources.extend(collect_files(&resource.path())?);
        } else {
            // File
            let file_path_buf = resource.path();
            let file_path = file_path_buf.to_string_lossy();
            if ACCEPTED_EXTENSION
                .iter()
                .all(|ext| !file_path.ends_with(ext))
            {
                continue;
            }

            all_resources.push(Resource {
                file_path: file_path_buf,
            })
        }
    }

    Ok(all_resources)
}
