/* xdg-utils library
 *
 * Copyright 2019-2020 Manos Pitsidianakis
 *
 * xdg-utils is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * xdg-utils is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with xdg-utils. If not, see <http://www.gnu.org/licenses/>.
 */

//! Query system for default apps using XDG MIME databases.
//!
//! The xdg-utils library provides dependency-free (except for `std`) Rust implementations of some
//! common functions in the freedesktop project `xdg-utils`.
//!
//! # What is implemented?
//! * Function <a class="fn" href="fn.query_default_app.html" title="xdg_utils::query_default_app fn">query_default_app</a> performs like the xdg-utils function `binary_to_desktop_file`
//! * Function <a class="fn" href="fn.query_mime_info.html" title="xdg_utils::query_mime_info fn">query_mime_info</a> launches the `mimetype` or else the `file` command.
//!
//! Some of the utils may be implemented by combining these functions with other functions in the Rust
//! standard library.
//!
//! | Name            | Function                                               | Implemented functionalities|
//! |-----------------|--------------------------------------------------------|----------------------------|
//! |`xdg-desktop-menu`| Install desktop menu items                             | no
//! |`xdg-desktop-icon`| Install icons to the desktop                           | no
//! |`xdg-icon-resource`| Install icon resources                                 | no
//! |`xdg-mime`        | Query information about file type handling and install descriptions for new file types| queries only
//! |`xdg-open`        | Open a file or URL in the user's preferred application | all (combine crate functions with `std::process::Command`)
//! |`xdg-email`       | Send mail using the user's preferred e-mail composer   | no
//! |`xdg-screensaver` | Control the screensaver                                | no
//!
//! # Specification
//! <https://specifications.freedesktop.org/mime-apps-spec/mime-apps-spec-latest.html>
//!
//! # Reference implementation
//! <https://cgit.freedesktop.org/xdg/xdg-utils/tree/scripts/xdg-utils-common.in>

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        /* Run with `cargo test -- --nocapture` to see output. */
        println!("{:?}", query_default_app("image/jpeg"));
        println!("{:?}", query_default_app("text/html"));
        println!("{:?}", query_default_app("video/mp4"));
        println!("{:?}", query_default_app("application/pdf"));
    }
}

use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::{Error, ErrorKind, Read, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str;

macro_rules! split_and_chain {
    ($xdg_vars:ident[$key:literal]) => {
        $xdg_vars.get($key).map(String::as_str).unwrap_or("").split(':')
    };
    ($xdg_vars:ident[$key:literal], $($tail_xdg_vars:ident[$tail_key:literal]),+$(,)*) => {

        split_and_chain!($xdg_vars[$key]).chain(split_and_chain!($($tail_xdg_vars[$tail_key]),+))
    }
}

/// Returns the path of a binary that is the default application of given MIME type `query`
///
/// # Example
/// ```no_run
/// use xdg_utils::query_default_app;
///
/// // The crate author recommends firefox.
/// assert_eq!(Ok("firefox".into()), query_default_app("text/html").map_err(|_| ()));
/// ```
pub fn query_default_app<T: AsRef<str>>(query: T) -> Result<PathBuf> {
    // Values are directory paths separated by : in case it's more than one.
    let mut xdg_vars: HashMap<String, String> = HashMap::new();
    let env_vars: env::Vars = env::vars();

    for (k, v) in env_vars {
        if k.starts_with("XDG_CONFIG")
            || k.starts_with("XDG_DATA")
            || k.starts_with("XDG_CURRENT_DESKTOP")
            || k == "HOME"
        {
            xdg_vars.insert(k.to_string(), v.to_string());
        }
    }

    // Insert defaults if variables are missing
    if xdg_vars.contains_key("HOME") && !xdg_vars.contains_key("XDG_DATA_HOME") {
        let h = xdg_vars["HOME"].clone();
        xdg_vars.insert("XDG_DATA_HOME".to_string(), format!("{}/.local/share", h));
    }

    if xdg_vars.contains_key("HOME") && !xdg_vars.contains_key("XDG_CONFIG_HOME") {
        let h = xdg_vars["HOME"].clone();
        xdg_vars.insert("XDG_CONFIG_HOME".to_string(), format!("{}/.config", h));
    }

    if !xdg_vars.contains_key("XDG_DATA_DIRS") {
        xdg_vars.insert(
            "XDG_DATA_DIRS".to_string(),
            "/usr/local/share:/usr/share".to_string(),
        );
    }

    let desktops: Option<Vec<&str>> = if xdg_vars.contains_key("XDG_CURRENT_DESKTOP") {
        let list = xdg_vars["XDG_CURRENT_DESKTOP"].trim().split(':').collect();
        Some(list)
    } else {
        None
    };

    // Search for mime entry in files.
    for p in split_and_chain!(
        xdg_vars["XDG_CONFIG_HOME"],
        xdg_vars["XDG_CONFIG_DIRS"],
        xdg_vars["XDG_DATA_HOME"],
        xdg_vars["XDG_DATA_DIRS"],
    ) {
        if let Some(ref d) = desktops {
            for desktop in d {
                let pb: PathBuf = PathBuf::from(format!(
                    "{var_value}/{desktop_val}-mimeapps.list",
                    var_value = p,
                    desktop_val = desktop
                ));
                if pb.exists() {
                    if let Some(ret) = check_mimeapps_list(&pb, &xdg_vars, &query)? {
                        return Ok(ret);
                    }
                }
            }
        }
        let pb: PathBuf = PathBuf::from(format!("{var_value}/mimeapps.list", var_value = p));
        if pb.exists() {
            if let Some(ret) = check_mimeapps_list(&pb, &xdg_vars, &query)? {
                return Ok(ret);
            }
        }
    }

    // Search again but for different paths.
    for p in split_and_chain!(xdg_vars["XDG_DATA_HOME"], xdg_vars["XDG_DATA_DIRS"]) {
        if let Some(ref d) = desktops {
            for desktop in d {
                let pb: PathBuf = PathBuf::from(format!(
                    "{var_value}/applications/{desktop_val}-mimeapps.list",
                    var_value = p,
                    desktop_val = desktop
                ));
                if pb.exists() {
                    if let Some(ret) = check_mimeapps_list(&pb, &xdg_vars, &query)? {
                        return Ok(ret);
                    }
                }
            }
        }
        let pb: PathBuf = PathBuf::from(format!(
            "{var_value}/applications/mimeapps.list",
            var_value = p
        ));
        if pb.exists() {
            if let Some(ret) = check_mimeapps_list(&pb, &xdg_vars, &query)? {
                return Ok(ret);
            }
        }
    }

    Err(Error::new(
        ErrorKind::NotFound,
        format!("No results for mime query: {}", query.as_ref()),
    ))
}

fn check_mimeapps_list<T: AsRef<str>>(
    filename: &Path,
    xdg_vars: &HashMap<String, String>,
    query: T,
) -> Result<Option<PathBuf>> {
    let mut file: File = File::open(filename)?;

    let mut contents: Vec<u8> = vec![];
    file.read_to_end(&mut contents)?;

    let contents_str =
        str::from_utf8(&contents).map_err(|err| Error::new(ErrorKind::InvalidData, err))?;
    let idx = contents_str.find(query.as_ref());

    if !contents_str.contains("[Default Applications]") || idx.is_none() {
        return Ok(None);
    }

    let idx = idx.unwrap();

    let mut end_idx = contents_str[idx..].len();
    for (cidx, c) in (&contents_str[idx..]).chars().enumerate() {
        if c == '\n' {
            end_idx = cidx + idx;
            break;
        }
    }

    let split_idx = if let Some(p) = contents_str[idx..end_idx].find('=') {
        p
    } else {
        /* Invalid data in in `filename`, but we don't want to abort the entire search for this
         * so just return None.
         */

        return Ok(None);
    } + idx
        + 1;

    for v in contents_str[split_idx..end_idx].split(';') {
        if v.trim().is_empty() {
            continue;
        }

        if let Some(b) = desktop_file_to_binary(v, xdg_vars)? {
            return Ok(Some(b));
        }
    }

    Ok(None)
}

// Find the desktop file in the filesystem, then find the binary it uses from its "Exec=..." line
// entry.
fn desktop_file_to_binary(
    desktop_name: &str,
    xdg_vars: &HashMap<String, String>,
) -> Result<Option<PathBuf>> {
    for dir in split_and_chain!(xdg_vars["XDG_DATA_HOME"], xdg_vars["XDG_DATA_DIRS"]) {
        let mut file_path: Option<PathBuf> = None;
        let mut p;
        if desktop_name.contains('-') {
            let v: Vec<&str> = desktop_name.split('-').collect();
            let (vendor, app): (&str, &str) = (v[0], v[1]);

            p = PathBuf::from(format!(
                "{dir}/applications/{vendor}/{app}",
                dir = dir,
                vendor = vendor,
                app = app
            ));
            if p.exists() {
                file_path = Some(p);
            } else {
                p = PathBuf::from(format!(
                    "{dir}/applnk/{vendor}/{app}",
                    dir = dir,
                    vendor = vendor,
                    app = app
                ));
                if p.exists() {
                    file_path = Some(p);
                }
            }
        }

        if file_path.is_none() {
            'indir: for indir in &[format!("{}/applications", dir), format!("{}/applnk", dir)] {
                p = PathBuf::from(format!(
                    "{indir}/{desktop}",
                    indir = indir,
                    desktop = desktop_name
                ));
                if p.exists() {
                    file_path = Some(p);
                    break 'indir;
                }
                p.pop(); // Remove {desktop} from path.
                if p.is_dir() {
                    for entry in fs::read_dir(&p)? {
                        let mut p = entry?.path().to_owned();
                        p.push(desktop_name);
                        if p.exists() {
                            file_path = Some(p);
                            break 'indir;
                        }
                    }
                }
            }
        }
        if let Some(file_path) = file_path {
            let mut f = fs::File::open(&file_path)?;
            let mut buf = vec![];
            f.read_to_end(&mut buf)?;
            let buf =
                str::from_utf8(&buf).map_err(|err| Error::new(ErrorKind::InvalidData, err))?;
            for l in buf.lines() {
                if l.starts_with("Exec") {
                    return Ok(l
                        .split('=')
                        .collect::<Vec<&str>>()
                        .get(1)
                        .map(|l| l.split(' ').collect::<Vec<&str>>())
                        .and_then(|l| l.get(0).map(PathBuf::from)));
                }
            }
        }
    }

    Ok(None)
}

/// Returns the MIME type of given file
/// https://cgit.freedesktop.org/xdg/xdg-utils/tree/scripts/xdg-mime.in
///
/// # Example
/// ```
/// use xdg_utils::query_mime_info;
/// let result = query_mime_info("/bin/sh")
///                 .map_err(|_| ())
///                 .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
/// let result_str = result.as_ref().map(|s| s.as_str());
/// assert!(Ok("application/x-pie-executable") == result_str || Ok("application/x-sharedlib") == result_str)
/// ```
pub fn query_mime_info<T: AsRef<Path>>(query: T) -> Result<Vec<u8>> {
    let command_obj = Command::new("mimetype")
        .args(&["--brief", "--dereference"])
        .arg(query.as_ref())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .or_else(|_| {
            Command::new("file")
                .args(&["--brief", "--dereference", "--mime-type"])
                .arg(query.as_ref())
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
        })?;

    Ok(drop_right_whitespace(
        command_obj.wait_with_output()?.stdout,
    ))
}

#[inline(always)]
fn drop_right_whitespace(mut vec: Vec<u8>) -> Vec<u8> {
    while vec.last() == Some(&b'\n') {
        vec.pop();
    }
    vec
}
