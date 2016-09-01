//! Extension methods for `docker_compose::v2::Service`.

use regex::Regex;
use std::path::{Path, PathBuf};

use docker_compose::v2 as dc;

/// These methods will appear as regular methods on `Service` in any module
/// which includes ServiceExt.
pub trait ServiceExt {
    /// Get the local build directory that we'll use for a service.
    /// Normally this will be based on its GitHub URL.
    fn local_build_dir(&self) ->
        Result<Option<PathBuf>, dc::Error>;
}

impl ServiceExt for dc::Service {
    fn local_build_dir(&self) ->
        Result<Option<PathBuf>, dc::Error>
    {
        if let Some(ref build) = self.build {
            Ok(Some(try!(git_to_local(try!(build.context.value())))))
        } else {
            Ok(None)
        }
    }
}

/// Given a build context, ensure that it points to a local directory.  If
/// this is a relative path, it will be relative to the `conductor` project
/// root.
fn git_to_local(ctx: &dc::Context) -> Result<PathBuf, dc::Error> {
    match ctx {
        &dc::Context::GitUrl(ref url) => {
            // Simulate a local checkout of the remote Git repository
            // mentioned in `build`.
            let re = Regex::new(r#"/([^./]+)(?:\.git)?$"#).unwrap();
            match re.captures(url) {
                None => Err(err!("Can't get dir name from Git URL: {}", url)),
                Some(caps) => {
                    let path = Path::new("src")
                        .join(caps.at(1).unwrap())
                        .to_owned();
                    Ok(path)
                }
            }
        }
        &dc::Context::Dir(ref dir) =>
            // Interpret `dir` relative to `pods` directory where we keep
            // our main `docker-compose.yml` files.
            Ok(Path::new("pods").join(dir).to_owned()),
    }
}

#[test]
fn git_to_local_fixes_local_directory_paths_as_needed() {
    let ctx = dc::Context::new("/src/foo");
    assert_eq!(git_to_local(&ctx).unwrap(),
               Path::new("/src/foo").to_owned());

    let ctx = dc::Context::new("../src/foo");
    assert_eq!(git_to_local(&ctx).unwrap(),
               Path::new("pods/../src/foo").to_owned());
}

#[test]
fn git_to_local_extracts_directory_part_of_git_urls() {
    let examples = &[
        // Example URLs from http://stackoverflow.com/a/34120821/12089,
        // originally from `docker-compose` source code.
        ("git://github.com/docker/docker", Some("docker")),
        ("git@github.com:docker/docker.git", Some("docker")),
        ("git@bitbucket.org:atlassianlabs/atlassian-docker.git",
         Some("atlassian-docker")),
        ("https://github.com/docker/docker.git", Some("docker")),
        ("http://github.com/docker/docker.git", Some("docker")),
        ("github.com/docker/docker.git", Some("docker")),
        // A URL from which we can't extract a local directory.
        ("http://www.example.com/", None),
    ];

    for &(url, dir) in examples {
        let in_ctx = dc::Context::new(url);
        if let Some(dir) = dir {
            let out_dir = Path::new("src").join(dir).to_owned();
            assert_eq!(git_to_local(&in_ctx).unwrap(), out_dir);
        } else {
            assert!(git_to_local(&in_ctx).is_err());
        }
    }
}
