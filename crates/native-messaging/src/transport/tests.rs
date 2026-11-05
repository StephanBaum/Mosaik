use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::*;
use crate::testutil::block_on;

/// A fresh endpoint for this platform: a pipe with a random name, or a
/// socket in a new private directory (kept alive by the returned guard).
fn fresh_endpoint() -> (Endpoint, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let endpoint = if cfg!(windows) {
        Endpoint::NamedPipe(crate::endpoint::new_windows_pipe_name().unwrap())
    } else {
        Endpoint::UnixSocket(dir.path().join("mosaic").join("browser.sock"))
    };
    (endpoint, dir)
}

#[test]
fn a_client_reaches_the_listener_and_bytes_flow() {
    block_on(async {
        let (endpoint, _dir) = fresh_endpoint();
        let mut listener = Listener::bind(&endpoint).unwrap();
        let server = tokio::spawn(async move {
            let mut stream = listener.accept().await.unwrap();
            let mut buf = [0u8; 5];
            stream.read_exact(&mut buf).await.unwrap();
            stream.write_all(&buf).await.unwrap();
            stream.flush().await.unwrap();
            (buf, listener)
        });
        let mut client = connect(&endpoint).await.unwrap();
        client.write_all(b"hello").await.unwrap();
        let mut echo = [0u8; 5];
        client.read_exact(&mut echo).await.unwrap();
        assert_eq!(&echo, b"hello");
        let (got, _listener) = server.await.unwrap();
        assert_eq!(&got, b"hello");
    });
}

#[test]
fn several_clients_are_served_in_turn() {
    block_on(async {
        let (endpoint, _dir) = fresh_endpoint();
        let mut listener = Listener::bind(&endpoint).unwrap();
        let server = tokio::spawn(async move {
            for _ in 0..3 {
                let mut stream = listener.accept().await.unwrap();
                let mut byte = [0u8; 1];
                stream.read_exact(&mut byte).await.unwrap();
                stream.write_all(&[byte[0] + 1]).await.unwrap();
                stream.flush().await.unwrap();
            }
        });
        for i in 0..3u8 {
            let mut client = connect(&endpoint).await.unwrap();
            client.write_all(&[i]).await.unwrap();
            let mut answer = [0u8; 1];
            client.read_exact(&mut answer).await.unwrap();
            assert_eq!(answer[0], i + 1);
        }
        server.await.unwrap();
    });
}

#[test]
fn a_second_listener_on_the_same_endpoint_is_refused() {
    block_on(async {
        let (endpoint, _dir) = fresh_endpoint();
        let _first = Listener::bind(&endpoint).unwrap();
        assert!(matches!(
            Listener::bind(&endpoint),
            Err(Error::EndpointInUse)
        ));
    });
}

#[test]
fn connecting_to_nothing_means_the_desktop_is_not_running() {
    block_on(async {
        let (endpoint, _dir) = fresh_endpoint();
        assert!(matches!(
            connect(&endpoint).await,
            Err(Error::DesktopNotRunning)
        ));
    });
}

#[test]
fn endpoints_of_the_other_platform_are_invalid() {
    block_on(async {
        let other = if cfg!(windows) {
            Endpoint::UnixSocket("/tmp/x/browser.sock".into())
        } else {
            Endpoint::NamedPipe(crate::endpoint::new_windows_pipe_name().unwrap())
        };
        assert!(matches!(
            Listener::bind(&other),
            Err(Error::EndpointInvalid)
        ));
        assert!(matches!(connect(&other).await, Err(Error::EndpointInvalid)));
    });
}

#[cfg(windows)]
#[test]
fn only_random_mosaic_pipe_names_are_accepted() {
    block_on(async {
        for name in [
            r"\\.\pipe\openssh-ssh-agent".to_owned(),
            r"\\.\pipe\mosaic-browser-short".to_owned(),
        ] {
            let endpoint = Endpoint::NamedPipe(name);
            assert!(matches!(
                Listener::bind(&endpoint),
                Err(Error::EndpointInvalid)
            ));
            assert!(matches!(
                connect(&endpoint).await,
                Err(Error::EndpointInvalid)
            ));
        }
    });
}

// The checks below need Unix permissions, owners and sockets. They are
// ported from the ssh-agent transport tests and the client rules of
// ADR-0036; they do not run on Windows.
#[cfg(unix)]
mod unix {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    use super::*;

    fn mode(path: &Path) -> u32 {
        use std::os::unix::fs::MetadataExt;
        fs::metadata(path).unwrap().mode() & 0o777
    }

    fn socket_in(dir: &Path) -> Endpoint {
        Endpoint::UnixSocket(dir.join("browser.sock"))
    }

    #[test]
    fn the_directory_is_created_private_and_the_socket_is_0600() {
        block_on(async {
            let root = tempfile::tempdir().unwrap();
            let dir = root.path().join("a").join("b");
            let endpoint = socket_in(&dir);
            let listener = Listener::bind(&endpoint).unwrap();
            assert_eq!(mode(&dir), 0o700);
            assert_eq!(mode(&dir.join("browser.sock")), 0o600);
            drop(listener);
            assert!(!dir.join("browser.sock").exists(), "removed on drop");
        });
    }

    #[test]
    fn a_shared_directory_is_refused() {
        block_on(async {
            let root = tempfile::tempdir().unwrap();
            for bad in [0o755, 0o770, 0o707, 0o750] {
                fs::set_permissions(root.path(), fs::Permissions::from_mode(bad)).unwrap();
                assert!(
                    matches!(
                        Listener::bind(&socket_in(root.path())),
                        Err(Error::EndpointInsecure)
                    ),
                    "{bad:o}"
                );
            }
        });
    }

    #[test]
    fn a_symbolic_link_directory_is_refused() {
        block_on(async {
            let root = tempfile::tempdir().unwrap();
            let real = root.path().join("real");
            fs::create_dir(&real).unwrap();
            fs::set_permissions(&real, fs::Permissions::from_mode(0o700)).unwrap();
            let link = root.path().join("link");
            std::os::unix::fs::symlink(&real, &link).unwrap();
            assert!(matches!(
                Listener::bind(&socket_in(&link)),
                Err(Error::EndpointInsecure)
            ));
        });
    }

    #[test]
    fn a_stale_socket_is_replaced_and_a_live_one_or_a_file_is_not() {
        block_on(async {
            let root = tempfile::tempdir().unwrap();
            fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
            let endpoint = socket_in(root.path());
            let path = root.path().join("browser.sock");

            // A socket nobody listens on (the listener was leaked).
            let stale = std::os::unix::net::UnixListener::bind(&path).unwrap();
            drop(stale);
            assert!(path.exists());
            let live = Listener::bind(&endpoint).unwrap();
            assert!(matches!(
                Listener::bind(&endpoint),
                Err(Error::EndpointInUse)
            ));
            drop(live);

            // Another kind of file is left alone.
            fs::write(&path, b"precious").unwrap();
            assert!(matches!(
                Listener::bind(&endpoint),
                Err(Error::EndpointInUse)
            ));
            assert_eq!(fs::read(&path).unwrap(), b"precious");
        });
    }

    #[test]
    fn long_and_relative_paths_are_invalid() {
        block_on(async {
            let long = Endpoint::UnixSocket(format!("/{}", "a".repeat(120)).into());
            assert!(matches!(Listener::bind(&long), Err(Error::EndpointInvalid)));
            assert!(matches!(connect(&long).await, Err(Error::EndpointInvalid)));
            let relative = Endpoint::UnixSocket("browser.sock".into());
            assert!(matches!(
                Listener::bind(&relative),
                Err(Error::EndpointInvalid)
            ));
        });
    }

    #[test]
    fn the_client_refuses_a_group_or_world_accessible_directory() {
        block_on(async {
            let root = tempfile::tempdir().unwrap();
            fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
            let endpoint = socket_in(root.path());
            let _listener = Listener::bind(&endpoint).unwrap();
            connect(&endpoint).await.unwrap();
            fs::set_permissions(root.path(), fs::Permissions::from_mode(0o750)).unwrap();
            assert!(matches!(
                connect(&endpoint).await,
                Err(Error::EndpointInsecure)
            ));
            fs::set_permissions(root.path(), fs::Permissions::from_mode(0o705)).unwrap();
            assert!(matches!(
                connect(&endpoint).await,
                Err(Error::EndpointInsecure)
            ));
        });
    }

    #[test]
    fn the_client_refuses_a_path_that_is_not_a_socket() {
        block_on(async {
            let root = tempfile::tempdir().unwrap();
            fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
            fs::write(root.path().join("browser.sock"), b"x").unwrap();
            assert!(matches!(
                connect(&socket_in(root.path())).await,
                Err(Error::EndpointInsecure)
            ));
        });
    }
}
