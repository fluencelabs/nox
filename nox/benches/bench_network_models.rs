/*
 * Copyright 2024 Fluence DAO
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

//     let services_config = ServicesConfig::new(
//         local_peer_id,
//         config_utils::services_dir(&tmp_dir),
//         config_utils::particles_vault_dir(&tmp_dir),
//         <_>::default(),
//         RandomPeerId::random(),
//         RandomPeerId::random(),
//     )
//     .wrap_err("create service config")
//     .unwrap();
//     let host_closures =
//         HostClosures::new(connectivity, node_info, services_config);
//
//     let pool_config = VmPoolConfig {
//         pool_size,
//         execution_timeout: TIMEOUT,
//     };
//     let vm_config = VmConfig {
//         current_peer_id: local_peer_id,
//         air_interpreter: interpreter,
//         particles_dir: config_utils::particles_dir(&tmp_dir),
//         particles_vault_dir: config_utils::particles_vault_dir(&tmp_dir),
//     };
//     let (stepper_pool, stepper_pool_api): (AquamarineBackend<AVM>, _) =
//         AquamarineBackend::new(pool_config, (vm_config, host_closures.descriptor()));
//
//     let handle = stepper_pool.start();
//
//     (stepper_pool_api, handle)
// }
//
// pub fn connectivity(
//     num_particles: usize,
//     current_peer_id: PeerId,
// ) -> (Connectivity, BoxFuture<'static, ()>, JoinHandle<()>) {
//     let (kademlia, kad_handle) = kademlia_api();
//     let (connection_pool, cp_handle) = connection_pool_api(num_particles, true);
//     let connectivity = Connectivity {
//         kademlia,
//         connection_pool,
//         current_peer_id,
//     };
//
//     (connectivity, cp_handle.boxed(), kad_handle)
// }
//
// pub fn connectivity_with_real_kad(
//     num_particles: usize,
//     network_size: usize,
//     current_peer_id: PeerId,
// ) -> (Connectivity, BoxFuture<'static, ()>, Stops, Vec<PeerId>) {
//     let (kademlia, stops, peer_ids) = real_kademlia_api(network_size);
//     let (connection_pool, cp_handle) = connection_pool_api(num_particles, false);
//     let connectivity = Connectivity {
//         kademlia,
//         connection_pool,
//         current_peer_id,
//     };
//
//     (connectivity, cp_handle.boxed(), stops, peer_ids)
// }
//
// pub async fn process_particles(
//     num_particles: usize,
//     parallelism: Option<usize>,
//     particle_timeout: Duration,
// ) {
//     let peer_id = RandomPeerId::random();
//     let (con, finish, kademlia) = connectivity(num_particles, peer_id);
//     let (aquamarine, aqua_handle) = aquamarine_api();
//     let (sink, _) = mpsc::unbounded();
//
//     let particle_stream: BackPressuredInlet<Particle> = particles(num_particles).await;
//     let process = spawn(con.clone().process_particles(
//         parallelism,
//         particle_stream,
//         aquamarine,
//         sink,
//         particle_timeout,
//     ));
//     finish.await;
//
//     process.cancel().await;
//     kademlia.cancel().await;
//     aqua_handle.cancel().await;
// }
//
// pub async fn process_particles_with_vm(
//     num_particles: usize,
//     pool_size: usize,
//     particle_parallelism: Option<usize>,
//     particle_timeout: Duration,
//     interpreter: PathBuf,
// ) {
//     let peer_id = RandomPeerId::random();
//
//     let (con, future, kademlia) = connectivity(num_particles, peer_id);
//     let (aquamarine, aqua_handle) =
//         aquamarine_with_vm(pool_size, con.clone(), peer_id, interpreter);
//     let (sink, _) = mpsc::unbounded();
//     let particle_stream: BackPressuredInlet<Particle> = particles(num_particles).await;
//     let process = spawn(con.clone().process_particles(
//         particle_parallelism,
//         particle_stream,
//         aquamarine,
//         sink,
//         particle_timeout,
//     ));
//     future.await;
//
//     process.cancel().await;
//     kademlia.cancel().await;
//     aqua_handle.cancel().await;
// }
//
// pub async fn process_particles_with_delay(
//     num_particles: usize,
//     pool_size: usize,
//     call_delay: Option<Duration>,
//     particle_parallelism: Option<usize>,
//     particle_timeout: Duration,
// ) {
//     let peer_id = RandomPeerId::random();
//
//     let (con, future, kademlia) = connectivity(num_particles, peer_id);
//     let (aquamarine, aqua_handle) = aquamarine_with_backend(pool_size, call_delay);
//     let (sink, _) = mpsc::unbounded();
//     let particle_stream: BackPressuredInlet<Particle> = particles(num_particles).await;
//     let process = spawn(con.clone().process_particles(
//         particle_parallelism,
//         particle_stream,
//         aquamarine,
//         sink,
//         particle_timeout,
//     ));
//     future.await;
//
//     process.cancel().await;
//     kademlia.cancel().await;
//     aqua_handle.cancel().await;
// }
