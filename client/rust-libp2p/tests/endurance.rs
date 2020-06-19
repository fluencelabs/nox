/*
 * Copyright 2020 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use config::{Config, File};
use faas_api::{peer, provider, Address, FunctionCall, Protocol};
use fluence_client::{Client, ClientEvent};
use fluence_libp2p::peerid_serializer;
use libp2p::PeerId;
use log::LevelFilter;
use parity_multiaddr::Multiaddr;
use rand::random;
use serde::Deserialize;
use std::iter::repeat;
use std::time::{Duration, Instant};

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);
const RECEIVE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug)]
struct Service {
    id: String,
    period: Duration,
    node: Node,
}

impl Service {
    pub fn new(period: Duration, node: Node) -> Self {
        use names::{Generator, Name};
        let mut generator = Generator::with_naming(Name::Plain);

        let name = generator.next().unwrap();
        #[rustfmt::skip]
        let name = name.split('-').next().unwrap().chars().take(10).collect::<String>();

        let id: String = format!("{:?}-{}", period, name);
        Self { id, period, node }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct Node {
    name: String,
    address: Multiaddr,
    #[serde(with = "peerid_serializer")]
    peer_id: PeerId,
}

#[derive(Debug, Deserialize)]
struct EnduranceConfig {
    nodes: Vec<Node>,
    client_multiplier: usize,
    intervals_minutes: Vec<u64>,
    log_level: LevelFilter,
}

fn load_config() -> EnduranceConfig {
    let mut config = Config::default();
    let file = File::with_name("endurance.toml").required(true);
    config
        .merge(file)
        .expect("Cannot load config endurance.toml");

    config
        .try_into()
        .expect("Cannot parse config endurance.toml")
}

#[ignore]
#[test]
#[rustfmt::skip]
fn endurance() {
    use Phenomena::*;
    use async_std::task;
    use async_timer::interval;
    use futures::future::join_all;
    
    let config = load_config();

    env_logger::builder().filter_level(config.log_level).init();

    let count = config.client_multiplier;
    let minutes = config.intervals_minutes; //vec![1, 2, 10, 30, 60, 120, 340, 600];

    let nodes = repeat(config.nodes.clone()).flatten();
    let services = minutes
        .into_iter()
        .map(|mins| Duration::from_secs(mins * 60))
        .flat_map(|p| repeat(p).take(count))
        .zip(nodes)
        .map(|(period, node)| Service::new(period, node))
        .collect::<Vec<_>>();

    let mut handles = vec![];

    for service in services {
        let prefix = service.id.clone(); 
        let nodes = config.nodes.clone();
        let handle = task::spawn(async move {
            let pause = random::<u64>() % 60;
            task::sleep(Duration::from_secs(pause)).await;

            let mut provider = match Client::connect(service.node.address.clone()).await {
                Ok((provider, _)) => provider,
                Err(e) => {
                    log::warn!("{: <14} - Provider didn't start: {:?}", prefix, e);
                    return
                },
            };
            
            log::info!("{: <14} - Provider waiting to connect to {:?}", prefix, service.node.address);

            if !wait_connected(&mut provider, &service.node.peer_id).await.success() {
                // exit if client stopped
                log::error!("{: <14} - Provider stopped before we connected to {:?}", prefix, service.node.address);
                report(ProviderConnectionFailed)
            } else {
                log::info!("{: <14} - Provider connected to {:?}", prefix, service.node.address);
                report(ProviderConnected)
            }

            loop {
                let mut periodic = interval(service.period);
                (&mut periodic).await;

                let reply_to = provider.relay_address(service.node.peer_id.clone());
                let node = peer!(service.node.peer_id.clone());
                provider.send(registration(reply_to.clone(), node, service.id.clone()), service.node.peer_id.clone());
                log::info!("{: <14} - Provider sent registration", prefix);
                task::sleep(Duration::from_secs(pause)).await;

                for node in nodes.clone() {
                    let mut consumer = match Client::connect(node.address.clone()).await {
                        Ok((consumer, _)) => consumer,
                        Err(e) => {
                            log::info!("{: <14}🌐 {: <8} ⇨ {: <8} - Consumer didn't connect: {:?}", prefix, node.name, service.node.name, e);
                            continue;
                        }
                    };
                    
                    if wait_connected(&mut consumer, &node.peer_id).await.success() {
                        log::info!("{: <14}🌐 {: <8} ⇨ {: <8} - Consumer connected", prefix, node.name, service.node.name);
                        report(ConsumerConnected);

                        let sent = Instant::now();
                        consumer.send(service_call(reply_to.clone(), service.id.clone()), node.peer_id.clone());
                        log::info!("{: <14}🌐 {: <8} ⇨ {: <8} - Consumer sent service call", prefix, node.name, service.node.name);

                        if wait_call(&mut provider, &service).await.success() {
                            log::info!("{: <14}🌐 {: <8} ⇨ {: <8} - Provider received call [{} micros]", prefix, node.name, service.node.name, sent.elapsed().as_micros());
                            report(ProviderReceived);
                        } else {
                            log::error!("{: <14}🌐 {: <8} ⇨ {: <8} - Provider didn't receive call", prefix, node.name, service.node.name);
                            report(ProviderReceiveFailed)
                        }

                        consumer.stop()
                    } else {
                        report(ConsumerConnectionFailed);

                        log::error!("{: <14}🌐 {: <8} ⇨ {: <8} - Consumer wasn't able to connect to {:?}", prefix, node.name, service.node.name, &node);
                    }
                }
            }
        });
        handles.push(handle);
    }

    task::block_on(join_all(handles));
}

enum Phenomena {
    ProviderConnected,
    ProviderConnectionFailed,
    ProviderReceived,
    ProviderReceiveFailed,
    ConsumerConnected,
    ConsumerConnectionFailed,
}

fn report(_phenomen: Phenomena) {}

enum Waiting<T> {
    Ok(T),
    ClientStopped,
    TimedOut,
}
impl<T> Waiting<T> {
    fn success(self) -> bool {
        matches!(self, Waiting::Ok(_))
    }
    #[allow(dead_code)]
    fn get(self) -> Option<T> {
        match self {
            Waiting::Ok(v) => Some(v),
            _ => None,
        }
    }
}
async fn wait_connected(client: &mut Client, expected_peer_id: &PeerId) -> Waiting<()> {
    use async_std::future::timeout;

    loop {
        match timeout(CONNECTION_TIMEOUT, client.receive_one()).await {
            Ok(Some(ClientEvent::NewConnection { peer_id, .. })) => {
                debug_assert_eq!(&peer_id, expected_peer_id);
                break Waiting::Ok(());
            }
            Ok(None) => break Waiting::ClientStopped,
            Ok(_) => continue,
            Err(_) => break Waiting::TimedOut,
        }
    }
}

async fn wait_call(client: &mut Client, expected_service: &Service) -> Waiting<FunctionCall> {
    use async_std::future::timeout;

    loop {
        match timeout(RECEIVE_TIMEOUT, client.receive_one()).await {
            Ok(Some(ClientEvent::FunctionCall { call, sender })) => {
                debug_assert_eq!(&sender, &expected_service.node.peer_id);
                let protocols = call.target.as_ref().map(|addr| addr.protocols());
                debug_assert!(matches!(
                    &protocols.as_deref(),
                    Some([Protocol::Providers(service_id)]) if service_id == &expected_service.id
                ));
                break Waiting::Ok(call);
            }
            Ok(None) => break Waiting::ClientStopped,
            Ok(_) => continue,
            Err(_) => break Waiting::TimedOut,
        }
    }
}

fn uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn service_call(sender: Address, service_id: String) -> FunctionCall {
    FunctionCall {
        uuid: uuid(),
        target: Some(provider!(service_id.clone())),
        module: Some(service_id),
        fname: None,
        arguments: serde_json::Value::Null,
        reply_to: Some(sender.clone()),
        name: Some("call service".into()),
        sender,
    }
}

fn registration(sender: Address, node: Address, service_id: String) -> FunctionCall {
    use serde_json::json;

    FunctionCall {
        uuid: uuid(),
        target: Some(node),
        module: Some("provide".into()),
        fname: None,
        reply_to: Some(sender.clone()),
        arguments: json!({ "service_id": service_id }),
        name: Some("registration".into()),
        sender,
    }
}
