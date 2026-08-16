use std::{
    io,
    mem::{self, MaybeUninit},
    net::{Ipv4Addr, SocketAddrV4},
    os::fd::{AsRawFd, RawFd},
    ptr,
};

use anyhow::{Context, Result, anyhow, ensure};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use tokio::{
    io::unix::AsyncFd,
    net::{TcpListener, UdpSocket},
};

#[derive(Debug)]
pub struct TransparentUdpSocket {
    socket: AsyncFd<Socket>,
}

impl TransparentUdpSocket {
    pub fn bind(port: u16, bypass_mark: u32) -> Result<Self> {
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_reuse_address(true)?;
        socket
            .set_ip_transparent_v4(true)
            .context("failed to enable IP_TRANSPARENT for UDP")?;
        socket.set_mark(bypass_mark)?;
        set_recv_original_destination(socket.as_raw_fd())?;
        socket.bind(&SockAddr::from(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            port,
        )))?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket: AsyncFd::new(socket)?,
        })
    }

    pub async fn recv_from_original(
        &self,
        buffer: &mut [u8],
    ) -> Result<(usize, SocketAddrV4, SocketAddrV4)> {
        loop {
            let mut readiness = self.socket.readable().await?;
            match readiness.try_io(|inner| recvmsg_original(inner.get_ref().as_raw_fd(), buffer)) {
                Ok(result) => return result.map_err(Into::into),
                Err(_) => continue,
            }
        }
    }
}

pub fn tcp_listener(port: u16, bypass_mark: u32) -> Result<TcpListener> {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
    socket.set_reuse_address(true)?;
    socket
        .set_ip_transparent_v4(true)
        .context("failed to enable IP_TRANSPARENT for TCP")?;
    socket.set_mark(bypass_mark)?;
    socket.bind(&SockAddr::from(SocketAddrV4::new(
        Ipv4Addr::UNSPECIFIED,
        port,
    )))?;
    socket.listen(1024)?;
    socket.set_nonblocking(true)?;
    let listener: std::net::TcpListener = socket.into();
    TcpListener::from_std(listener).context("failed to create async transparent TCP listener")
}

pub fn udp_response_socket(source: SocketAddrV4, bypass_mark: u32) -> Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    socket
        .set_ip_transparent_v4(true)
        .context("failed to enable IP_TRANSPARENT for UDP response socket")?;
    socket.set_mark(bypass_mark)?;
    socket
        .bind(&SockAddr::from(source))
        .with_context(|| format!("failed to bind transparent UDP response source {source}"))?;
    socket.set_nonblocking(true)?;
    let socket: std::net::UdpSocket = socket.into();
    UdpSocket::from_std(socket).context("failed to create async UDP response socket")
}

pub fn set_mark(fd: RawFd, mark: u32) -> io::Result<()> {
    let result = unsafe {
        // SAFETY: fd is borrowed for the duration of the call and the value pointer is valid.
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_MARK,
            (&mark as *const u32).cast(),
            mem::size_of_val(&mark) as libc::socklen_t,
        )
    };
    if result == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn set_recv_original_destination(fd: RawFd) -> io::Result<()> {
    let enabled: libc::c_int = 1;
    let result = unsafe {
        // SAFETY: fd is borrowed for the duration of the call and enabled is a valid c_int.
        libc::setsockopt(
            fd,
            libc::IPPROTO_IP,
            libc::IP_RECVORIGDSTADDR,
            (&enabled as *const libc::c_int).cast(),
            mem::size_of_val(&enabled) as libc::socklen_t,
        )
    };
    if result == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn recvmsg_original(
    fd: RawFd,
    buffer: &mut [u8],
) -> io::Result<(usize, SocketAddrV4, SocketAddrV4)> {
    let mut source = MaybeUninit::<libc::sockaddr_in>::zeroed();
    let mut control = [0_usize; 8];
    let mut iov = libc::iovec {
        iov_base: buffer.as_mut_ptr().cast(),
        iov_len: buffer.len(),
    };
    let mut message: libc::msghdr = unsafe { mem::zeroed() };
    message.msg_name = source.as_mut_ptr().cast();
    message.msg_namelen = mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
    message.msg_iov = &mut iov;
    message.msg_iovlen = 1;
    message.msg_control = control.as_mut_ptr().cast();
    message.msg_controllen = mem::size_of_val(&control) as _;

    let received = unsafe {
        // SAFETY: message points to initialized writable buffers for recvmsg.
        libc::recvmsg(fd, &mut message, 0)
    };
    if received == -1 {
        return Err(io::Error::last_os_error());
    }
    if message.msg_flags & libc::MSG_CTRUNC != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "original-destination control message was truncated",
        ));
    }

    let source = unsafe {
        // SAFETY: a successful IPv4 recvmsg populated msg_name with sockaddr_in.
        source.assume_init()
    };
    if source.sin_family as libc::c_int != libc::AF_INET {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "received a non-IPv4 UDP packet",
        ));
    }
    let source = sockaddr_to_v4(source);

    let mut destination = None;
    let mut current = unsafe {
        // SAFETY: message has a valid control buffer initialized by recvmsg.
        libc::CMSG_FIRSTHDR(&message)
    };
    while !current.is_null() {
        let header = unsafe {
            // SAFETY: current is returned by CMSG_FIRSTHDR/CMSG_NXTHDR for message.
            &*current
        };
        if header.cmsg_level == libc::IPPROTO_IP && header.cmsg_type == libc::IP_ORIGDSTADDR {
            let address = unsafe {
                // SAFETY: IP_ORIGDSTADDR carries a sockaddr_in; read_unaligned handles CMSG layout.
                ptr::read_unaligned(libc::CMSG_DATA(current).cast::<libc::sockaddr_in>())
            };
            destination = Some(sockaddr_to_v4(address));
            break;
        }
        current = unsafe {
            // SAFETY: current belongs to message and the libc helper validates bounds.
            libc::CMSG_NXTHDR(&message, current)
        };
    }

    let destination = destination.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "UDP packet did not include IP_ORIGDSTADDR",
        )
    })?;
    Ok((received as usize, source, destination))
}

fn sockaddr_to_v4(address: libc::sockaddr_in) -> SocketAddrV4 {
    SocketAddrV4::new(
        Ipv4Addr::from(address.sin_addr.s_addr.to_ne_bytes()),
        u16::from_be(address.sin_port),
    )
}

pub fn original_tcp_target(socket: &tokio::net::TcpStream) -> Result<SocketAddrV4> {
    match socket.local_addr()? {
        std::net::SocketAddr::V4(address) => {
            ensure!(
                address.port() != 0,
                "transparent TCP target has a zero port"
            );
            Ok(address)
        }
        std::net::SocketAddr::V6(_) => Err(anyhow!("IPv6 transparent connections are unsupported")),
    }
}
