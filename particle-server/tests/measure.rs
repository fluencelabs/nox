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

use serde_json::json;
use std::thread::sleep;
use std::time::Instant;
use test_utils::{make_swarms_with_cfg, ConnectedClient, KAD_TIMEOUT};

/*
(call ("12D3KooWEXNUbCXooUwHrHBbrmjsrpHXoEphPwbjQXEGyzbqKnE9" ("identity" "") () void[]))
(call ("12D3KooWHk9BjDQBUqnavciRPhAYFvqKBe4ZiPPvde7vDaqgn5er" ("identity" "") () void[]))
(call ("12D3KooWBUJifCTgaxAUrcM9JysqCcS4CS8tiYH5hExbdWCAoNwb" ("identity" "") () void[]))
(call ("12D3KooWJbJFaZ3k5sNd8DjQgg3aERoKtBAnirEvPV8yp76kEXHB" ("identity" "") () void[]))
(call ("12D3KooWCKCeqLPSgMnDjyFsJuWqREDtKNHx1JEBiwaMXhCLNTRb" ("identity" "") () void[]))
(call ("12D3KooWMhVpgfQxBLkQkJed8VFNvgN4iE6MD7xCybb1ZYWW2Gtz" ("identity" "") () void[]))
(call ("12D3KooWPnLxnY71JDxvB3zbjKu9k1BCYNthGZw6iGrLYsR1RnWM" ("identity" "") () void[]))
*/

#[test]
fn measure() {
    // let swarms = make_swarms_with_cfg(3, |cfg| cfg);
    // sleep(KAD_TIMEOUT);
    // let mut a = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");
    // let mut b = ConnectedClient::connect_to(swarms[1].1.clone()).expect("connect client");
    let mut a = ConnectedClient::connect_to("/ip4/134.209.186.43/tcp/9001/ws".parse().unwrap())
        .expect("connect client");
    let mut b = ConnectedClient::connect_to("/ip4/134.209.186.43/tcp/9002/ws".parse().unwrap())
        .expect("connect client");

    let mut i = 10;
    loop {
        i -= 1;
        if i < 0 {
            break;
        }
        let now = Instant::now();
        a.send_particle(
            format!(
                r#"
                (seq (
                    (call ("12D3KooWEXNUbCXooUwHrHBbrmjsrpHXoEphPwbjQXEGyzbqKnE9" ("identity" "") () void[]))
                    (seq (
                        (call ("12D3KooWHk9BjDQBUqnavciRPhAYFvqKBe4ZiPPvde7vDaqgn5er" ("identity" "") () void[]))
                        (seq (
                            (call ("12D3KooWBUJifCTgaxAUrcM9JysqCcS4CS8tiYH5hExbdWCAoNwb" ("identity" "") () void[]))
                            (seq (
                                (call ("12D3KooWJbJFaZ3k5sNd8DjQgg3aERoKtBAnirEvPV8yp76kEXHB" ("identity" "") () void[]))
                                (seq (
                                    (call ("12D3KooWCKCeqLPSgMnDjyFsJuWqREDtKNHx1JEBiwaMXhCLNTRb" ("identity" "") () void[]))
                                    (seq (
                                        (call ("12D3KooWMhVpgfQxBLkQkJed8VFNvgN4iE6MD7xCybb1ZYWW2Gtz" ("identity" "") () void[]))
                                        (seq (
                                            (call ("12D3KooWPnLxnY71JDxvB3zbjKu9k1BCYNthGZw6iGrLYsR1RnWM" ("identity" "") () void[]))
                                            (seq (
                                                (call ("{}" ("identity" "") () void[]))
                                                (call ("{}" ("identity" "") () void[]))
                                            ))
                                        ))
                                    ))
                                ))
                            ))
                        ))
                    ))
                ))
                "#,
                b.node, b.peer_id
            ),
            json!({}),
        );

        b.receive();

        println!("relay elapsed: {:?}", now.elapsed());
    }

    let mut i = 10;
    loop {
        i -= 1;
        if i < 0 {
            break;
        }
        let now = Instant::now();
        a.send_particle(
            format!(
                r#"
                    (seq (
                        (call ("{}" ("identity" "") () void1))
                    ))
                "#,
                a.peer_id
            ),
            json!({}),
        );

        a.receive();

        println!("echo elapsed: {:?}", now.elapsed());
    }
}
