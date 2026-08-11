use std::io;
use std::net::{Ipv4Addr, TcpListener};

const MIXED_PORT_FALLBACKS: &[u16] = &[17890, 17891, 27890, 27891, 37997];

pub(crate) fn is_loopback_tcp_port_available(port: u16) -> bool {
    port != 0 && probe_loopback_tcp_port(port).is_ok()
}

pub(crate) fn choose_available_mixed_port(requested: u16) -> io::Result<u16> {
    if let Some(port) = choose_available_mixed_port_with(requested, MIXED_PORT_FALLBACKS, probe_loopback_tcp_port)? {
        return Ok(port);
    }

    // The fixed candidates can all be unavailable on hosts with large Windows
    // excluded-port ranges. Let the OS choose a final usable loopback port.
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

fn probe_loopback_tcp_port(port: u16) -> io::Result<()> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
    drop(listener);
    Ok(())
}

fn choose_available_mixed_port_with<F>(requested: u16, fallbacks: &[u16], mut probe: F) -> io::Result<Option<u16>>
where
    F: FnMut(u16) -> io::Result<()>,
{
    if requested != 0 {
        match probe(requested) {
            Ok(()) => return Ok(Some(requested)),
            // A running Clash Verge service can legitimately own the port
            // while the UI process starts. Preserve that configured port.
            Err(err) if err.kind() == io::ErrorKind::AddrInUse => return Ok(Some(requested)),
            // WSAEACCES is what Windows returns for an excluded/reserved port.
            Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {}
            Err(err) => return Err(err),
        }
    }

    for port in fallbacks.iter().copied().filter(|port| *port != 0) {
        if probe(port).is_ok() {
            return Ok(Some(port));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::choose_available_mixed_port_with;
    use std::io;

    #[test]
    fn keeps_requested_port_when_available() {
        let selected = choose_available_mixed_port_with(7897, &[17890], |port| {
            if port == 7897 {
                Ok(())
            } else {
                Err(io::ErrorKind::AddrInUse.into())
            }
        })
        .unwrap();

        assert_eq!(selected, Some(7897));
    }

    #[test]
    fn preserves_port_owned_by_running_service() {
        let selected =
            choose_available_mixed_port_with(7897, &[17890], |_| Err(io::ErrorKind::AddrInUse.into())).unwrap();

        assert_eq!(selected, Some(7897));
    }

    #[test]
    fn selects_first_available_fallback_for_permission_denied() {
        let selected = choose_available_mixed_port_with(7897, &[17890, 17891, 27890], |port| match port {
            7897 => Err(io::ErrorKind::PermissionDenied.into()),
            17891 => Ok(()),
            _ => Err(io::ErrorKind::AddrInUse.into()),
        })
        .unwrap();

        assert_eq!(selected, Some(17891));
    }

    #[test]
    fn ignores_zero_and_reports_no_fixed_candidate() {
        let selected =
            choose_available_mixed_port_with(0, &[0, 17890], |_| Err(io::ErrorKind::AddrInUse.into())).unwrap();

        assert_eq!(selected, None);
    }
}
