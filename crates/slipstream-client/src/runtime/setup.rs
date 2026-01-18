use crate::error::ClientError;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

#[cfg(not(windows))]
use std::net::{Ipv6Addr, SocketAddrV6};
use tokio::net::UdpSocket as TokioUdpSocket;

pub(crate) fn compute_mtu(domain_len: usize) -> Result<u32, ClientError> {
    if domain_len >= 240 {
        return Err(ClientError::new(
            "Domain name is too long for DNS transport",
        ));
    }
    let mut mtu = ((240.0 - domain_len as f64) / 1.6) as u32;
    #[cfg(windows)]
    {
        // Windows UDP send can fail with WSAEMSGSIZE; keep a conservative cap.
        mtu = mtu.min(512);
    }
    if mtu == 0 {
        return Err(ClientError::new(
            "MTU computed to zero; check domain length",
        ));
    }
    Ok(mtu)
}

pub(crate) async fn bind_udp_socket() -> Result<TokioUdpSocket, ClientError> {
    #[cfg(windows)]
    {
        let bind_v4 = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0));
        return TokioUdpSocket::bind(bind_v4).await.map_err(map_io);
    }

    #[cfg(not(windows))]
    {
        let bind_v6 = SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0));
        return TokioUdpSocket::bind(bind_v6).await.map_err(map_io);
    }
}

pub(crate) fn map_io(err: std::io::Error) -> ClientError {
    ClientError::new(err.to_string())
}
