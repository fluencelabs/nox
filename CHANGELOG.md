# Changelog

## [0.8.1](https://github.com/fluencelabs/rust-peer/compare/rust-peer-v0.8.0...rust-peer-v0.8.1) (2023-02-24)


### Features

* **workers:** allow to deploy decider to root scope, fix aliasing ([#1488](https://github.com/fluencelabs/rust-peer/issues/1488)) ([193a6e7](https://github.com/fluencelabs/rust-peer/commit/193a6e7ff4af40ea1d90b42a6aff3629e86f52ac))


### Bug Fixes

* builtin redeploy and associated aliases bug ([#1486](https://github.com/fluencelabs/rust-peer/issues/1486)) ([2c11e35](https://github.com/fluencelabs/rust-peer/commit/2c11e3569c5bc35c3857335e1040b64abe3deedb))
* **deps:** Bump spell to 0.5.0 ([#1483](https://github.com/fluencelabs/rust-peer/issues/1483)) ([b03da8b](https://github.com/fluencelabs/rust-peer/commit/b03da8b1e866d9ffa0887e4fc48b9ecc109799ee))
* **keypairs:** load persisted keypair in a backward-compatible way ([#1481](https://github.com/fluencelabs/rust-peer/issues/1481)) ([ab51b9c](https://github.com/fluencelabs/rust-peer/commit/ab51b9cf32f0bf83776617d8c81113eb51285d8c))
* **metrics:** fix call metrics collection for aliased services ([#1487](https://github.com/fluencelabs/rust-peer/issues/1487)) ([0e7b76e](https://github.com/fluencelabs/rust-peer/commit/0e7b76e0f16847f7af554d784dd9ab316d9fc063))
* **spell:** update spell to 0.5.2 ([#1489](https://github.com/fluencelabs/rust-peer/issues/1489)) ([d79a0da](https://github.com/fluencelabs/rust-peer/commit/d79a0da72ff2c0cd4491f30fde477d95b97b0342))

## [0.8.0](https://github.com/fluencelabs/rust-peer/compare/rust-peer-v0.7.4...rust-peer-v0.8.0) (2023-02-22)


### ⚠ BREAKING CHANGES

* implement per-worker aliasing [NET-352] ([#1449](https://github.com/fluencelabs/rust-peer/issues/1449))

### Features

* add worker.create and worker.get_peer_id ([#1475](https://github.com/fluencelabs/rust-peer/issues/1475)) ([ddc2f90](https://github.com/fluencelabs/rust-peer/commit/ddc2f901cc2578386bae765e8a2029339d0fa801))
* **builtins:** impl srv.info, dist.get_blueprint ([#1468](https://github.com/fluencelabs/rust-peer/issues/1468)) ([8d21afa](https://github.com/fluencelabs/rust-peer/commit/8d21afa7d6e22b81f557c011adc3b4b3eb6a6055))
* implement per-worker aliasing [NET-352] ([#1449](https://github.com/fluencelabs/rust-peer/issues/1449)) ([097d47d](https://github.com/fluencelabs/rust-peer/commit/097d47dda079587281f312a8066e8f86652b9da5))
* **kademlia:** Forbid to use Provider API ([#1466](https://github.com/fluencelabs/rust-peer/issues/1466)) ([2eb068f](https://github.com/fluencelabs/rust-peer/commit/2eb068fcefa27b7e9b477229cd58761bec75e2d4))
* **libp2p:** Optional async std libp2p & debug profile ([#1454](https://github.com/fluencelabs/rust-peer/issues/1454)) ([1aab9f7](https://github.com/fluencelabs/rust-peer/commit/1aab9f71bb7811a122d73d2588fe4e16d8eecc7b))
* **sig:** add insecure_sig ([#1458](https://github.com/fluencelabs/rust-peer/issues/1458)) ([9727c3f](https://github.com/fluencelabs/rust-peer/commit/9727c3f8522ea7caea5c570cfed768903ff7f4c9))
* **sig:** sig use worker keypair if called in worker scope [NET-371] ([#1455](https://github.com/fluencelabs/rust-peer/issues/1455)) ([e45655b](https://github.com/fluencelabs/rust-peer/commit/e45655bc565df7c7c967954699bcdbd7d52ef99b))
* **spells:** support -relay- for spell scripts [NET-374] ([#1461](https://github.com/fluencelabs/rust-peer/issues/1461)) ([b7c3427](https://github.com/fluencelabs/rust-peer/commit/b7c342710a1e607d9af50f233c4d2ca5b0cda87b))


### Bug Fixes

* **connection-pool:** fixed logging ([#1463](https://github.com/fluencelabs/rust-peer/issues/1463)) ([66e364e](https://github.com/fluencelabs/rust-peer/commit/66e364ed022d033fccef8a1b4f9007992c566098))
* **deps:** update rust crate fluence-spell-distro to 0.3.2 ([#1464](https://github.com/fluencelabs/rust-peer/issues/1464)) ([d15c084](https://github.com/fluencelabs/rust-peer/commit/d15c084ef92ad009be28634b26b86d14228d4cb3))
* **deps:** update rust crate fluence-spell-dtos to 0.3.1 ([#1462](https://github.com/fluencelabs/rust-peer/issues/1462)) ([72b426d](https://github.com/fluencelabs/rust-peer/commit/72b426d68de22fd8ebfef8378d26736362b25cae))
* **deps:** update rust crate fluence-spell-dtos to 0.3.2 ([#1465](https://github.com/fluencelabs/rust-peer/issues/1465)) ([4c13688](https://github.com/fluencelabs/rust-peer/commit/4c136883d6733e9a68d249ac1044437a5e950bc4))
* remove worker_id check for service calls ([#1459](https://github.com/fluencelabs/rust-peer/issues/1459)) ([ebfbf34](https://github.com/fluencelabs/rust-peer/commit/ebfbf34faf9ad431e377a179250db4e18ae2ca8d))
* revert [#1459](https://github.com/fluencelabs/rust-peer/issues/1459) and really fix worker id check for service calls ([#1460](https://github.com/fluencelabs/rust-peer/issues/1460)) ([2978847](https://github.com/fluencelabs/rust-peer/commit/2978847bbd5fa72e05dfc8c969b61c538ba184cb))
* **spells:** allow global aliases for builtin spells ([#1477](https://github.com/fluencelabs/rust-peer/issues/1477)) ([e87b77c](https://github.com/fluencelabs/rust-peer/commit/e87b77cade5235c6a50dc376ed886a051e7e0d6a))
* **spells:** do not run spells with passed (ended) Clock trigger ([#1452](https://github.com/fluencelabs/rust-peer/issues/1452)) ([fe2184a](https://github.com/fluencelabs/rust-peer/commit/fe2184af05bbe19123d58d0aba2a46750e891098))

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
