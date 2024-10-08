use std::{future::Future, io::Result, net::SocketAddr};

use tokio::net::{TcpListener, TcpStream};

impl super::Listener for TcpListener {
    type Io = TcpStream;
    type Addr = SocketAddr;

    fn accept(&self) -> impl Future<Output = Result<(Self::Io, Self::Addr)>> + Send {
        Self::accept(self)
    }

    fn local_addr(&self) -> Result<Self::Addr> {
        Self::local_addr(self)
    }
}
