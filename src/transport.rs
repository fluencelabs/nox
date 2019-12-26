use libp2p::{
    core, dns, identity, mplex, secio, tcp, websocket, yamux, PeerId, Transport, TransportError,
};
use libp2p_core::Multiaddr;
use std::time::Duration;
use std::{error, io};

pub fn build_mplex(
    keypair: identity::Keypair,
) -> impl Transport<
    Output = (
        PeerId,
        impl core::muxing::StreamMuxer<
                OutboundSubstream = impl Send,
                Substream = impl Send,
                Error = impl Into<io::Error>,
            > + Send
            + Sync,
    ),
    Error = impl error::Error + Send,
    Listener = impl Send,
    Dial = impl Send,
    ListenerUpgrade = impl Send,
> + Clone {
    CommonTransport::new()
        .upgrade(core::upgrade::Version::V1)
        .authenticate(secio::SecioConfig::new(keypair))
        .multiplex(mplex::MplexConfig::new())
        .map(|(peer, muxer), _| (peer, core::muxing::StreamMuxerBox::new(muxer)))
        .timeout(Duration::from_secs(20))
}

#[derive(Debug, Clone)]
pub struct CommonTransport {
    // The actual implementation of everything.
    inner: CommonTransportInner,
}

#[cfg(all(
    not(any(target_os = "emscripten", target_os = "unknown")),
    feature = "libp2p-websocket"
))]
type InnerImplementation = core::transport::OrTransport<
    dns::DnsConfig<tcp::TcpConfig>,
    websocket::WsConfig<dns::DnsConfig<tcp::TcpConfig>>,
>;
#[cfg(all(
    not(any(target_os = "emscripten", target_os = "unknown")),
    not(feature = "libp2p-websocket")
))]
type InnerImplementation = dns::DnsConfig<tcp::TcpConfig>;
#[cfg(any(target_os = "emscripten", target_os = "unknown"))]
type InnerImplementation = core::transport::dummy::DummyTransport;

#[derive(Debug, Clone)]
struct CommonTransportInner {
    inner: InnerImplementation,
}

impl CommonTransport {
    /// Initializes the `CommonTransport`.
    #[cfg(not(any(target_os = "emscripten", target_os = "unknown")))]
    pub fn new() -> CommonTransport {
        let tcp = tcp::TcpConfig::new().nodelay(true);
        let transport = dns::DnsConfig::new(tcp);
        #[cfg(feature = "libp2p-websocket")]
        let transport = {
            let trans_clone = transport.clone();
            transport.or_transport(websocket::WsConfig::new(trans_clone))
        };

        CommonTransport {
            inner: CommonTransportInner { inner: transport },
        }
    }

    /// Initializes the `CommonTransport`.
    #[cfg(any(target_os = "emscripten", target_os = "unknown"))]
    pub fn new() -> CommonTransport {
        let inner = core::transport::dummy::DummyTransport::new();
        CommonTransport {
            inner: CommonTransportInner { inner },
        }
    }
}

impl Transport for CommonTransport {
    type Output = <InnerImplementation as Transport>::Output;
    type Error = <InnerImplementation as Transport>::Error;
    type Listener = <InnerImplementation as Transport>::Listener;
    type ListenerUpgrade = <InnerImplementation as Transport>::ListenerUpgrade;
    type Dial = <InnerImplementation as Transport>::Dial;

    fn listen_on(self, addr: Multiaddr) -> Result<Self::Listener, TransportError<Self::Error>> {
        self.inner.inner.listen_on(addr)
    }

    fn dial(self, addr: Multiaddr) -> Result<Self::Dial, TransportError<Self::Error>> {
        self.inner.inner.dial(addr)
    }
}
