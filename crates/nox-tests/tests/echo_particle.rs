/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use connected_client::ConnectedClient;
use created_swarm::make_swarms;

use eyre::WrapErr;
use maplit::hashmap;
use serde_json::json;

#[tokio::test]
async fn echo_particle() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let data = hashmap! {
        "name" => json!("folex"),
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
    };
    let response = client
        .execute_particle(
            r#"
        (seq
            (call relay ("op" "noop") [])
            (call client ("return" "") [name])
        )"#,
            data.clone(),
        )
        .await
        .unwrap();
    assert_eq!(data["name"], response[0]);
}
