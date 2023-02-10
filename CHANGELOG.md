# Changelog

## [0.7.4](https://github.com/fluencelabs/rust-peer/compare/rust-peer-v0.7.3...rust-peer-v0.7.4) (2023-02-09)


### Features

* **builtins:** json.obj_pairs, json.puts_pairs to build JSON objects from array of pairs ([#1434](https://github.com/fluencelabs/rust-peer/issues/1434)) ([3539f67](https://github.com/fluencelabs/rust-peer/commit/3539f677308b833b2a9897d50a660b5ce038ad98))
* **connection-pool:** Add libp2p connection limits ([#1437](https://github.com/fluencelabs/rust-peer/issues/1437)) ([090ac84](https://github.com/fluencelabs/rust-peer/commit/090ac84627c3ab915e9932887659155aa9bab669))
* **spells:** do json manipulations internally ([#1433](https://github.com/fluencelabs/rust-peer/issues/1433)) ([ff36b61](https://github.com/fluencelabs/rust-peer/commit/ff36b6144f129ed071edab610d980ca017a4c94c))


### Bug Fixes

* **connection-pool:** Keep only fluence peers in the connection pool ([#1440](https://github.com/fluencelabs/rust-peer/issues/1440)) ([546bc6f](https://github.com/fluencelabs/rust-peer/commit/546bc6f6a390790b461bb82450fc713b908f287c))
* **deps:** update libp2p to 0.50.0 [fixes [NET-232](https://linear.app/fluence/issue/NET-232)] [#1419](https://github.com/fluencelabs/rust-peer/issues/1419) ([9c3eda8](https://github.com/fluencelabs/rust-peer/commit/9c3eda83883ef7e21e5a4fb82c40b9f955aa2936))
* **deps:** update rust crate air-interpreter-wasm to v0.35.4 ([#1447](https://github.com/fluencelabs/rust-peer/issues/1447)) ([b8de74d](https://github.com/fluencelabs/rust-peer/commit/b8de74d693bfad4ced4e57c2e390e6552c6baac8))
* **deps:** update rust crate avm-server to 0.28.1 ([#1420](https://github.com/fluencelabs/rust-peer/issues/1420)) ([03887af](https://github.com/fluencelabs/rust-peer/commit/03887af8f89ae5683faa2eed1cadb94df2d7ef22))
* **spells:** Allow parallel execution of immut service functions [NET-331] ([#1430](https://github.com/fluencelabs/rust-peer/issues/1430)) ([e9a05d6](https://github.com/fluencelabs/rust-peer/commit/e9a05d6a8a402e0cfb7955a110ad79a15450ab1c))
* **spells:** correct interpretation of end_sec field [NET-346] ([#1431](https://github.com/fluencelabs/rust-peer/issues/1431)) ([2b018de](https://github.com/fluencelabs/rust-peer/commit/2b018dedeec4bcebf7d34ecfb47c6167e8c7f6ef))

## [0.7.3](https://github.com/fluencelabs/rust-peer/compare/rust-peer-v0.7.2...rust-peer-v0.7.3) (2023-01-20)


### Bug Fixes

* **ci:** set permissions for release job ([f5842a8](https://github.com/fluencelabs/rust-peer/commit/f5842a8c087092e69251e8d0f1b88cab79b0349f))

## [0.7.2](https://github.com/fluencelabs/rust-peer/compare/rust-peer-v0.7.1...rust-peer-v0.7.2) (2023-01-18)


### Features

* **spells:** support empty trigger configs [NET-316] ([#1412](https://github.com/fluencelabs/rust-peer/issues/1412)) ([46d8fd5](https://github.com/fluencelabs/rust-peer/commit/46d8fd5763a1122406e11bc89a63edc7babeb081))
* support keypairs for spells [fixes NET-237, NET-239, NET-281 and NET-283] [#1382](https://github.com/fluencelabs/rust-peer/issues/1382) ([63d0759](https://github.com/fluencelabs/rust-peer/commit/63d07597d5fdc875adec1ade2b1010969422c87f))
