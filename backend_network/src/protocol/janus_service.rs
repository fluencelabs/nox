use crate::protocol::config::Config;
use crate::protocol::janus_events;

use futures::{Stream, Poll};
use libp2p::{
    dns::DnsConfig, identity, mplex::MplexConfig, secio::SecioConfig, tcp::TcpConfig, PeerId, Swarm,
};
use libp2p_core::identity::Keypair;
use libp2p_core::{muxing::StreamMuxerBox, nodes::Substream, transport::boxed::Boxed, upgrade, Multiaddr};
use slog::Logger;
use std::io::Error;
use tokio::prelude::{AsyncRead, AsyncWrite};
use crate::protocol::janus_events::JanusEvent;

//type JanusTransport = Boxed<(PeerId, StreamMuxerBox), Error>;
//type JanusBehaviour = Behaviour<Substream<StreamMuxerBox>>;

pub struct JanusService<TTransport, TBehaviour> {
    pub swarm: Swarm<TTransport, TBehaviour>,

    pub local_peer_id: PeerId,

    pub logger: Logger,
}

impl<TTransport, TBehaviour> JanusService<TTransport, TBehaviour>
    where TTransport: AsyncRead + AsyncWrite,
          TBehaviour: AsyncRead + AsyncWrite {

    pub fn new(config: Config, logger: Logger) -> Self {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        let mut swarm = {
            let transport = build_transport();

            let behaviour = JanusBehaviour::new();
            Swarm::new(transport, behaviour, local_peer_id.clone())
        };

        let mut listen_addr = Multiaddr::from(config.listen_ip);
        listen_addr.push(Protocol::Tcp(config.listen_port));

        Swarm::listen_on(&mut swarm, listen_addr).unwrap();

        for topics in config.topics {
            let raw_topic: Topic = topic.into();
            let topic_string = raw_topic.no_hash();
            swarm.subscribe(raw_topic).expect("subscribe failed");
        };

        JanusService {
            swarm,
            local_peer_id,
            logger,
        }
    }

    fn build_transport(local_private_key: Keypair) -> TTransport {
        let transport = TcpConfig::new().nodelay(true);
        let transport = DnsConfig::new(transport);

        transport
            .upgrade(upgrade::Version::V1)
            .authenticate(SecioConfig::new(local_private_key))
            .multiplex(MplexConfig::new())
            .map(|(p, m), _| (p, core::muxing::StreamMuxerBox::new(m)))
            .unwrap()
            .boxed()
    }
}

impl<TTransport, TBehaviour> Stream for JanusService<TTransport, TBehaviour>
    where TTransport: AsyncRead + AsyncWrite,
          TBehaviour: AsyncRead + AsyncWrite {
    type Item = JanusEvent;
    type Error = std::io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            match self.swarm.poll() {

            }
        }
    }
}

