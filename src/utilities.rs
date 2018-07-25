use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str;

/// Returns the path of a binary that is the default application of MIME type `query`
pub fn query_default_app<T: AsRef<str>>(query: T) -> Result<PathBuf, ()> {
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
        let list = xdg_vars["XDG_CURRENT_DESKTOP"].trim().split(":").collect();
        Some(list)
    } else {
        None
    };

    // Search for mime entry in files.
    for v in &[
        xdg_vars.get("XDG_CONFIG_HOME"),
        xdg_vars.get("XDG_CONFIG_DIRS"),
        xdg_vars.get("XDG_DATA_HOME"),
        xdg_vars.get("XDG_DATA_DIRS"),
    ] {
        if let Some(v) = v {
            for p in v.split(":") {
                if let Some(ref d) = desktops {
                    for desktop in d {
                        let pb: PathBuf = PathBuf::from(format!(
                            "{var_value}/{desktop_val}-mimeapps.list",
                            var_value = p,
                            desktop_val = desktop
                        ));
                        if pb.exists() {
                            if let Ok(ret) = check_mimeapps_list(&pb, &xdg_vars, &query) {
                                return Ok(ret);
                            }
                        }
                    }
                    let pb: PathBuf =
                        PathBuf::from(format!("{var_value}/mimeapps.list", var_value = p));
                    if pb.exists() {
                        if let Ok(ret) = check_mimeapps_list(&pb, &xdg_vars, &query) {
                            return Ok(ret);
                        }
                    }
                }
            }
        }
    }

    // Search again but for different paths.
    for v in &[xdg_vars.get("XDG_DATA_HOME"), xdg_vars.get("XDG_DATA_DIRS")] {
        if let Some(v) = v {
            for p in v.split(":") {
                if let Some(ref d) = desktops {
                    for desktop in d {
                        let pb: PathBuf = PathBuf::from(format!(
                            "{var_value}/applications/{desktop_val}-mimeapps.list",
                            var_value = p,
                            desktop_val = desktop
                        ));
                        if pb.exists() {
                            if let Ok(ret) = check_mimeapps_list(&pb, &xdg_vars, &query) {
                                return Ok(ret);
                            }
                        }
                    }
                    let pb: PathBuf = PathBuf::from(format!(
                        "{var_value}/applications/mimeapps.list",
                        var_value = p
                    ));
                    if pb.exists() {
                        if let Ok(ret) = check_mimeapps_list(&pb, &xdg_vars, &query) {
                            return Ok(ret);
                        }
                    }
                }
            }
        }
    }

    // Nothing found.
    return Err(());
}

fn check_mimeapps_list<T: AsRef<str>>(
    filename: &Path,
    xdg_vars: &HashMap<String, String>,
    query: T,
) -> Result<PathBuf, ()> {
    let mut file: File = match File::open(filename) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "mime-apps: mimeapps.list in {} could not be opened.\nError returned: {}",
                filename.display(),
                e
            );
            return Err(());
        }
    };

    let mut contents: Vec<u8> = vec![];
    if file.read_to_end(&mut contents).is_err() {
        eprintln!(
            "mime-apps: mimeapps.list in {} could not be read",
            filename.display()
        );
        return Err(());
    }
    let contents_str = match str::from_utf8(&contents) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "mime-apps: mimeapps.list in {} is not vald UTF-8.\nError returned: {}",
                filename.display(),
                e
            );
            return Err(());
        }
    };

    let idx = contents_str.find(query.as_ref());

    if !contents_str.contains("[Default Applications]") || idx.is_none() {
        return Err(());
    }

    let idx = idx.unwrap();

    let mut end_idx = contents_str[idx..].len();
    for (cidx, c) in (&contents_str[idx..]).chars().enumerate() {
        if c == '\n' {
            end_idx = cidx + idx;
            break;
        }
    }

    let split_idx = contents_str[idx..end_idx].find("=").unwrap() + idx + 1;

    for v in contents_str[split_idx..end_idx].split(";") {
        if v.trim().len() == 0 {
            continue;
        }

        if let Ok(b) = desktop_file_to_binary(v, xdg_vars) {
            return Ok(b);
        }
    }

    Err(())
}

// Find the desktop file in the filesystem, then find the binary it uses from its "Exec=..." line
// entry.
fn desktop_file_to_binary(
    desktop_name: &str,
    xdg_vars: &HashMap<String, String>,
) -> Result<PathBuf, ()> {
    'dir_a: for dir_a in &[xdg_vars.get("XDG_DATA_HOME"), xdg_vars.get("XDG_DATA_DIRS")] {
        if let Some(dir_b) = dir_a {
            'dir_b: for dir in dir_b.split(":") {
                let mut file_path: Option<PathBuf> = None;
                let mut p;
                if desktop_name.contains("-") {
                    let v: Vec<&str> = desktop_name.split("-").collect();
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
                    'indir: for indir in
                        &[format!("{}/applications", dir), format!("{}/applnk", dir)]
                    {
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
                            for entry in fs::read_dir(&p).unwrap() {
                                let mut p = entry.unwrap().path().to_owned();
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
                    let mut f = fs::File::open(&file_path).unwrap();
                    let mut buf = vec![];
                    f.read_to_end(&mut buf).unwrap();
                    let mut buf = str::from_utf8(&buf).unwrap();
                    for l in buf.lines() {
                        if l.starts_with("Exec") {
                            let l: Vec<&str> = l.split("=").collect();
                            let l = l.get(1).unwrap();
                            let l: Vec<&str> = l.split(" ").collect();
                            return Ok(PathBuf::from(l[0]));
                        }
                    }
                }
            }
        }
    }

    Err(())
}
