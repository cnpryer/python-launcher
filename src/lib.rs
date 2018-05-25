use std::collections;
use std::env;
use std::path;

/// An integer part of a version specifier (e.g. the `X or `Y of `X.Y`).
type VersionComponent = u16;

/// Represents the version of Python a user requsted.
#[derive(Debug, PartialEq)]
pub enum RequestedVersion {
    Any,
    Loose(VersionComponent),
    Exact(VersionComponent, VersionComponent),
}

impl RequestedVersion {
    /// Creates a new `RequestedVersion` from a version specifier string.
    fn from_string(ver: &String) -> Result<Self, String> {
        let mut char_iter = ver.chars();
        let mut major_ver: Vec<char> = Vec::new();
        let mut dot = false;
        for c in char_iter.by_ref() {
            if c == '.' {
                dot = true;
                break;
            } else if c.is_ascii_digit() {
                major_ver.push(c);
            } else {
                return Err(format!(
                    "{:?} contains a non-numeric and non-period character",
                    ver
                ));
            }
        }

        let mut minor_ver: Vec<char> = Vec::new();
        if dot {
            for c in char_iter.by_ref() {
                if c.is_ascii_digit() {
                    minor_ver.push(c);
                } else {
                    return Err(format!(
                        "{:?} contains a non-numeric character after a period",
                        ver
                    ));
                }
            }
        }

        if major_ver.len() == 0 {
            Err(format!("version string is empty"))
        } else {
            let major = char_vec_to_int(&major_ver)?;
            if !dot {
                Ok(RequestedVersion::Loose(major))
            } else if minor_ver.len() == 0 {
                Err(format!("{:?} is missing a minor version number", ver))
            } else {
                let minor = char_vec_to_int(&minor_ver)?;
                Ok(RequestedVersion::Exact(major, minor))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Version {
    major: VersionComponent,
    minor: VersionComponent,
}

/*
enum VersionMatch {
    NotAtAll,
    Loosely,
    Exactly,
}
*/

/// Converts a `Vec<char>` to a `VersionComponent` integer.
fn char_vec_to_int(char_vec: &Vec<char>) -> Result<VersionComponent, String> {
    let joined_string = char_vec.into_iter().collect::<String>();
    let parse_result = joined_string.parse::<VersionComponent>();
    parse_result.or(Err(format!(
        "error converting {:?} to a number",
        joined_string
    )))
}

/// Attempts to parse a version specifier from a CLI argument.
///
/// Any failure to parse leads to `RequestedVersion::Any` being returned.
fn parse_version_from_cli(arg: &String) -> RequestedVersion {
    if arg.starts_with("-") {
        let mut version = arg.clone();
        version.remove(0);
        match RequestedVersion::from_string(&version) {
            Ok(v) => v,
            Err(_) => RequestedVersion::Any,
        }
    } else {
        RequestedVersion::Any
    }
}

/// Checks if the string contains a version specifier.
///
/// If not version specifier is found, `RequestedVersion::Any` is returned.
//
// https://docs.python.org/3.8/using/windows.html#from-the-command-line
pub fn check_cli_arg(arg: &String) -> RequestedVersion {
    let version_from_cli = parse_version_from_cli(arg);
    if version_from_cli != RequestedVersion::Any {
        version_from_cli
    } else {
        // XXX shebang from file
        println!("No version found in the first CLI arg");
        RequestedVersion::Any
    }
}

/// Returns the entries in `PATH`.
fn path_entries() -> Vec<path::PathBuf> {
    let path_val = match env::var_os("PATH") {
        Some(val) => val,
        None => return Vec::new(),
    };
    env::split_paths(&path_val).collect()
}

/// Gets the files contained in the directory.
fn directory_contents(path: &path::PathBuf) -> collections::HashSet<path::PathBuf> {
    let mut files = collections::HashSet::new();
    if let Ok(contents) = path.read_dir() {
        for content in contents {
            if let Ok(found_content) = content {
                let path = found_content.path();
                if path.is_file() {
                    files.insert(path);
                }
            }
        }
    }

    files
}

/// Filters the file paths down to `pythonX.Y` files.
fn filter_python_executables(
    paths: collections::HashSet<path::PathBuf>,
) -> collections::HashMap<Version, path::PathBuf> {
    let mut executables = collections::HashMap::new();
    for path in paths {
        let unencoded_file_name = match path.file_name() {
            Some(x) => x,
            None => continue,
        };
        let file_name = match unencoded_file_name.to_str() {
            Some(x) => x,
            None => continue,
        };
        if file_name.len() < "python3.0".len() || !file_name.starts_with("python") {
            continue;
        }
        let version_part = &file_name["python".len()..];
        if let Ok(found_version) = RequestedVersion::from_string(&version_part.to_string()) {
            match found_version {
                RequestedVersion::Exact(major, minor) => executables.insert(
                    Version {
                        major: major,
                        minor: minor,
                    },
                    path.clone(),
                ),
                _ => continue,
            };
        }
    }

    return executables;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(non_snake_case)]
    fn test_RequestedVersion_from_string() {
        assert!(RequestedVersion::from_string(&".3".to_string()).is_err());
        assert!(RequestedVersion::from_string(&"3.".to_string()).is_err());
        assert!(RequestedVersion::from_string(&"h".to_string()).is_err());
        assert!(RequestedVersion::from_string(&"3.b".to_string()).is_err());
        assert!(RequestedVersion::from_string(&"a.7".to_string()).is_err());
        assert_eq!(
            RequestedVersion::from_string(&"3".to_string()),
            Ok(RequestedVersion::Loose(3))
        );
        assert_eq!(
            RequestedVersion::from_string(&"3.8".to_string()),
            Ok(RequestedVersion::Exact(3, 8))
        );
        assert_eq!(
            RequestedVersion::from_string(&"42.13".to_string()),
            Ok(RequestedVersion::Exact(42, 13))
        );
        assert!(RequestedVersion::from_string(&"3.6.5".to_string()).is_err());
    }

    #[test]
    fn test_parse_version_from_cli() {
        assert_eq!(
            parse_version_from_cli(&"path/to/file".to_string()),
            RequestedVersion::Any
        );
        assert_eq!(
            parse_version_from_cli(&"3".to_string()),
            RequestedVersion::Any
        );
        assert_eq!(
            parse_version_from_cli(&"-S".to_string()),
            RequestedVersion::Any
        );
        assert_eq!(
            parse_version_from_cli(&"--something".to_string()),
            RequestedVersion::Any
        );
        assert_eq!(
            parse_version_from_cli(&"-3".to_string()),
            RequestedVersion::Loose(3)
        );
        assert_eq!(
            parse_version_from_cli(&"-3.6".to_string()),
            RequestedVersion::Exact(3, 6)
        );
        assert_eq!(
            parse_version_from_cli(&"-42.13".to_string()),
            RequestedVersion::Exact(42, 13)
        );
        assert_eq!(
            parse_version_from_cli(&"-3.6.4".to_string()),
            RequestedVersion::Any
        );
    }

    #[test]
    fn unit_test_path_entries() {
        let paths = vec!["/a", "/b", "/c"];
        if let Ok(joined_paths) = env::join_paths(&paths) {
            let original_paths = env::var_os("PATH");
            env::set_var("PATH", joined_paths);
            assert_eq!(
                path_entries(),
                paths
                    .iter()
                    .map(|p| path::PathBuf::from(p))
                    .collect::<Vec<path::PathBuf>>()
            );
            match original_paths {
                Some(paths) => env::set_var("PATH", paths),
                None => env::set_var("PATH", ""),
            }
        }
    }

    #[test]
    fn system_test_path_entries() {
        if let Some(paths) = env::var_os("PATH") {
            let found_paths = path_entries();
            assert_eq!(found_paths.len(), env::split_paths(&paths).count());
            for (index, path) in env::split_paths(&paths).enumerate() {
                assert_eq!(found_paths[index], path);
            }
        }
    }

    #[test]
    fn test_filter_python_executables() {
        let paths = vec![
            "/bad/path/python",    // Under-specified.
            "/bad/path/python3",   // Under-specified.
            "/bad/path/hello",     // Not Python.
            "/bad/path/pytho3.6",  // Typo.
            "/bad/path/rython3.6", // Typo.
            "/good/path/python3.6",
            "/good/python42.13",
        ];
        let all_paths = paths
            .iter()
            .map(|p| path::PathBuf::from(p))
            .collect::<collections::HashSet<path::PathBuf>>();
        let results = filter_python_executables(all_paths);
        let good_version1 = Version { major: 3, minor: 6 };
        let good_version2 = Version {
            major: 42,
            minor: 13,
        };
        let mut expected = paths[5];
        match results.get(&good_version1) {
            Some(path) => assert_eq!(*path, path::PathBuf::from(expected)),
            None => panic!("{:?} not found", good_version1),
        };
        expected = paths[6];
        match results.get(&good_version2) {
            Some(path) => assert_eq!(*path, path::PathBuf::from(expected)),
            None => panic!("{:?} not found", good_version2),
        }
    }
}
