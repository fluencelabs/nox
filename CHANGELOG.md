# Changelog

## [0.14.7](https://github.com/fluencelabs/nox/compare/rust-peer-v0.14.6...rust-peer-v0.14.7) (2023-09-21)


### Features

* **builtins:** implement subnet.resolve [NET-549] ([#1798](https://github.com/fluencelabs/nox/issues/1798)) ([85665e2](https://github.com/fluencelabs/nox/commit/85665e2caa7f1dd668576be5a41125d02c38f577))


### Bug Fixes

* **deps:** update rust crate fluence-spell-distro to v0.5.20 ([#1781](https://github.com/fluencelabs/nox/issues/1781)) ([9e380ac](https://github.com/fluencelabs/nox/commit/9e380acc03a3b4e015d052c6969bfdb2055db1d0))
* **deps:** update rust crate fluence-spell-dtos to v0.5.20 ([#1782](https://github.com/fluencelabs/nox/issues/1782)) ([09c9b49](https://github.com/fluencelabs/nox/commit/09c9b4967dbcbfc9010884ec0c7e29f5bcde8ccb))

## [0.14.6](https://github.com/fluencelabs/nox/compare/rust-peer-v0.14.5...rust-peer-v0.14.6) (2023-09-15)


### Features

* **system-services, tests:** composable and overridable system services [NET-530, NET-531] ([#1770](https://github.com/fluencelabs/nox/issues/1770)) ([459ec72](https://github.com/fluencelabs/nox/commit/459ec72a0c9bbcd93ec7a12b4eb097dbda2ee40f))


### Bug Fixes

* **deps:** update rust crate air-interpreter-wasm to 0.47.0 ([#1785](https://github.com/fluencelabs/nox/issues/1785)) ([33ac736](https://github.com/fluencelabs/nox/commit/33ac7361a8d749dea5e9b8372ee3930a0929582f))

## [0.14.5](https://github.com/fluencelabs/nox/compare/rust-peer-v0.14.4...rust-peer-v0.14.5) (2023-09-07)


### Bug Fixes

* **deps:** update rust crate air-interpreter-wasm to 0.46.0 ([#1776](https://github.com/fluencelabs/nox/issues/1776)) ([2c94c72](https://github.com/fluencelabs/nox/commit/2c94c72030e23aeeebe4c40bf700b7000125ae4d))

## [0.14.4](https://github.com/fluencelabs/nox/compare/rust-peer-v0.14.3...rust-peer-v0.14.4) (2023-09-06)


### Features

* update decider ([#1780](https://github.com/fluencelabs/nox/issues/1780)) ([f4419f7](https://github.com/fluencelabs/nox/commit/f4419f743c8e988b010681778af196ac65b38bc5))

## [0.14.3](https://github.com/fluencelabs/nox/compare/rust-peer-v0.14.2...rust-peer-v0.14.3) (2023-09-06)


### Features

* **worker:** allow to add alias for worker creator ([#1779](https://github.com/fluencelabs/nox/issues/1779)) ([ada1276](https://github.com/fluencelabs/nox/commit/ada127679d5e1bd38a55bad5fa5d9ec72cc67cc1))


### Bug Fixes

* **deps:** update rust crate avm-server to 0.33.0 ([#1777](https://github.com/fluencelabs/nox/issues/1777)) ([98ea570](https://github.com/fluencelabs/nox/commit/98ea570461ac0fe5172f18aeddd4110545c539d0))
* **deps:** update rust crate fluence-spell-distro to v0.5.18 ([#1774](https://github.com/fluencelabs/nox/issues/1774)) ([c51ae71](https://github.com/fluencelabs/nox/commit/c51ae715451d66fc14484b3a0e7a266c6e781bdf))
* **deps:** update rust crate fluence-spell-dtos to v0.5.18 ([#1775](https://github.com/fluencelabs/nox/issues/1775)) ([99233c3](https://github.com/fluencelabs/nox/commit/99233c334293cd5bbd6e4f5ff1f2909ec5538e0d))

## [0.14.2](https://github.com/fluencelabs/nox/compare/rust-peer-v0.14.1...rust-peer-v0.14.2) (2023-09-05)


### Features

* **builtins:** add resolve_alias_opt [NET-528] ([#1769](https://github.com/fluencelabs/nox/issues/1769)) ([b352cad](https://github.com/fluencelabs/nox/commit/b352cad8837e3e53616eb4857bb3768d03fcc129))
* **decider:** new decider ([#1764](https://github.com/fluencelabs/nox/issues/1764)) ([cd49dd2](https://github.com/fluencelabs/nox/commit/cd49dd2b76bc1aec647cb188329162ca09e94e1e))
* **spells:** add opt `alias` arg to spell install [NET-529] ([#1772](https://github.com/fluencelabs/nox/issues/1772)) ([fe8eab2](https://github.com/fluencelabs/nox/commit/fe8eab2ab98269fdee23574901f51b4dafa66140))
* **workers:** add Worker.get_worker_id [NET-523] ([#1771](https://github.com/fluencelabs/nox/issues/1771)) ([a45cf79](https://github.com/fluencelabs/nox/commit/a45cf799200abbf2884fe4881b10ad85a20c6a18))

## [0.14.1](https://github.com/fluencelabs/nox/compare/rust-peer-v0.14.0...rust-peer-v0.14.1) (2023-08-14)


### Bug Fixes

* **http:** fixed listen address getting logic ([#1762](https://github.com/fluencelabs/nox/issues/1762)) ([1205a79](https://github.com/fluencelabs/nox/commit/1205a79c6ad3ef8df60010964aafcfb3af80ad10))

## [0.14.0](https://github.com/fluencelabs/nox/compare/rust-peer-v0.13.3...rust-peer-v0.14.0) (2023-08-14)


### ⚠ BREAKING CHANGES

* **env, log:** RUST_LOG and other envs parsing ([#1755](https://github.com/fluencelabs/nox/issues/1755))

### Bug Fixes

* **config:** fix http port argument ([#1751](https://github.com/fluencelabs/nox/issues/1751)) ([84c4315](https://github.com/fluencelabs/nox/commit/84c43151693a4bfcabc7683d9b3c6b4e508ff427))
* **env, log:** RUST_LOG and other envs parsing ([#1755](https://github.com/fluencelabs/nox/issues/1755)) ([ea172c1](https://github.com/fluencelabs/nox/commit/ea172c1f42823549aeef0ba5ebd41794682babdf))

## [0.13.3](https://github.com/fluencelabs/nox/compare/rust-peer-v0.13.2...rust-peer-v0.13.3) (2023-08-08)


### Features

* **http:** Added http healthcheck endpoint ([#1725](https://github.com/fluencelabs/nox/issues/1725)) ([ba2c9d1](https://github.com/fluencelabs/nox/commit/ba2c9d118049d944a0eb7cd1d3e8fda598af65cf))
* **spells:** store last trigger by `trigger` key in KV [NET-511] ([#1728](https://github.com/fluencelabs/nox/issues/1728)) ([7272040](https://github.com/fluencelabs/nox/commit/7272040b37de66c32bd6f35e5c5e505bdaad40af))
* **tests:** Wait for healtcheck in nox tests ([#1739](https://github.com/fluencelabs/nox/issues/1739)) ([2c4748c](https://github.com/fluencelabs/nox/commit/2c4748c4d2a61da8a1f2cde58758a8c793a095b7))


### Bug Fixes

* **deps:** update rust crate air-interpreter-wasm to 0.43.1 ([#1750](https://github.com/fluencelabs/nox/issues/1750)) ([4f94c2c](https://github.com/fluencelabs/nox/commit/4f94c2c708c6c49367607d5d66a453451ae7fb56))
* **deps:** update rust crate avm-server to 0.32.2 ([#1744](https://github.com/fluencelabs/nox/issues/1744)) ([d6569d0](https://github.com/fluencelabs/nox/commit/d6569d09db61d29e1af570128288e5042bfb3b6f))
* **spell:** bump spell to 0.5.17 and fix tests ([#1724](https://github.com/fluencelabs/nox/issues/1724)) ([671881c](https://github.com/fluencelabs/nox/commit/671881c0c74d319249c098eead06d672fa6230c0))

## [0.13.2](https://github.com/fluencelabs/nox/compare/rust-peer-v0.13.1...rust-peer-v0.13.2) (2023-07-20)


### Features

* **builtins:** add vault.cat, vault.put + refactoring [NET-489 NET-491] ([#1631](https://github.com/fluencelabs/nox/issues/1631)) ([18fb419](https://github.com/fluencelabs/nox/commit/18fb41967b9ceec0d0d23085bd889abbbac90059))
* **builtins:** remove builtins dir from configs ([#1711](https://github.com/fluencelabs/nox/issues/1711)) ([9bce547](https://github.com/fluencelabs/nox/commit/9bce547b1b330d6f66784ca3c432f6ac634472c4))
* **decider:** update to 0.4.17 ([#1702](https://github.com/fluencelabs/nox/issues/1702)) ([453470d](https://github.com/fluencelabs/nox/commit/453470d881940719a78b168aed6107e1418c7f33))
* **http:** added versions endpoint ([#1700](https://github.com/fluencelabs/nox/issues/1700)) ([ada94e5](https://github.com/fluencelabs/nox/commit/ada94e51025e2499fde4dd934c740e1e155246e9))
* **makefile:** introduce local-env to run local aurora & ipfs ([#1701](https://github.com/fluencelabs/nox/issues/1701)) ([9199ab1](https://github.com/fluencelabs/nox/commit/9199ab1747cafaf63e68435697a242ffced0fd0d))
* **metrics:** added metrics and peer_id endpoints ([#1699](https://github.com/fluencelabs/nox/issues/1699)) ([c42b919](https://github.com/fluencelabs/nox/commit/c42b9193fdb015c1fb8031c2a62edabc2c354986))
* **spell:** update to new mailbox [NET-505] ([#1694](https://github.com/fluencelabs/nox/issues/1694)) ([15f1921](https://github.com/fluencelabs/nox/commit/15f1921f31ee73e285e5ff5d32cbe7ac6c7a29e5))
* **tracing:** introduce deal_id-based trace logs to particle execution ([#1656](https://github.com/fluencelabs/nox/issues/1656)) ([760e7c8](https://github.com/fluencelabs/nox/commit/760e7c8b83a75782397e610e945963e17b13ff8e))


### Bug Fixes

* **deps:** update rust crate air-interpreter-wasm to 0.41.0 ([#1659](https://github.com/fluencelabs/nox/issues/1659)) ([be8d5c6](https://github.com/fluencelabs/nox/commit/be8d5c6ffe2a93a378b506161059606e0214278c))
* **deps:** update rust crate air-interpreter-wasm to 0.42.0 ([#1709](https://github.com/fluencelabs/nox/issues/1709)) ([fd8bc24](https://github.com/fluencelabs/nox/commit/fd8bc24c48001d410d8b0d6f206af818e78e852c))
* **error:** log error on accepting incoming connection ([#1657](https://github.com/fluencelabs/nox/issues/1657)) ([06f7668](https://github.com/fluencelabs/nox/commit/06f766852a0dc152067e04f9160a89ec29336c78))
* **system-services:** allow overriding binary paths for system services [fixes NET-503] ([#1663](https://github.com/fluencelabs/nox/issues/1663)) ([d4ff356](https://github.com/fluencelabs/nox/commit/d4ff3564b57e39de8d1625b4fd934412495bdbe1))
* **system-services:** update system spell without redeploying [fixes NET-490] ([#1649](https://github.com/fluencelabs/nox/issues/1649)) ([0d4f833](https://github.com/fluencelabs/nox/commit/0d4f833fefc5ce8624a0f7e7dcc7d85292025027))

## [0.13.1](https://github.com/fluencelabs/nox/compare/rust-peer-v0.13.0...rust-peer-v0.13.1) (2023-06-27)


### Bug Fixes

* **avm:** avm 0.40, avm-server 0.32 ([#1647](https://github.com/fluencelabs/nox/issues/1647)) ([777f6e1](https://github.com/fluencelabs/nox/commit/777f6e1c3b7f21695b8b41da820a0033f417c342))

## [0.13.0](https://github.com/fluencelabs/nox/compare/rust-peer-v0.12.1...rust-peer-v0.13.0) (2023-06-22)


### ⚠ BREAKING CHANGES

* **avm:** update to avm 0.39.1 ([#1627](https://github.com/fluencelabs/nox/issues/1627))

### Features

* **avm:** update to avm 0.39.1 ([#1627](https://github.com/fluencelabs/nox/issues/1627)) ([e6b1afa](https://github.com/fluencelabs/nox/commit/e6b1afa0417d873cf94c4646a723d267e51fdfba))
* Migrate Registry to spell ([#1629](https://github.com/fluencelabs/nox/issues/1629)) ([978ea2c](https://github.com/fluencelabs/nox/commit/978ea2cf64015e47027fd64a60dee2f06a7b3cab))
* **system-service-deployer:** introduce new system service deployment system [fixes NET-487] ([#1623](https://github.com/fluencelabs/nox/issues/1623)) ([f272c02](https://github.com/fluencelabs/nox/commit/f272c02434a7f20df3ec2258e47e47570b90cbbd))

## [0.12.1](https://github.com/fluencelabs/rust-peer/compare/rust-peer-v0.12.0...rust-peer-v0.12.1) (2023-05-22)


### Bug Fixes

* **async:** move builtins call to blocking threadpool ([#1621](https://github.com/fluencelabs/rust-peer/issues/1621)) ([7e18801](https://github.com/fluencelabs/rust-peer/commit/7e188014e9bafbe5d6d9b15c6e62ea7b4b879b27))

## [0.12.0](https://github.com/fluencelabs/rust-peer/compare/rust-peer-v0.11.4...rust-peer-v0.12.0) (2023-05-18)


### ⚠ BREAKING CHANGES

* **dist:** make blueprint an IPLD, use CID as module ids [fixes NET-412 NET-432] ([#1595](https://github.com/fluencelabs/rust-peer/issues/1595))
* **config:** Introduce tracing config, change allowed_binaries list format ([#1608](https://github.com/fluencelabs/rust-peer/issues/1608))

### Features

* **config:** Introduce tracing config, change allowed_binaries list format ([#1608](https://github.com/fluencelabs/rust-peer/issues/1608)) ([6f7c684](https://github.com/fluencelabs/rust-peer/commit/6f7c68462dd69eb1f8b60a5b18f29fbd43884cd3))
* **dist:** make blueprint an IPLD, use CID as module ids [fixes NET-412 NET-432] ([#1595](https://github.com/fluencelabs/rust-peer/issues/1595)) ([720a2b3](https://github.com/fluencelabs/rust-peer/commit/720a2b353bd9ae7766329c5b02e3f2a34b37b903))
* **log:** change 'particle received' log to info ([0047edf](https://github.com/fluencelabs/rust-peer/commit/0047edfa60a06345aebbb5c5ed53de793963b08b))
* **log:** change 'particle received' log to info [NET-472] ([#1616](https://github.com/fluencelabs/rust-peer/issues/1616)) ([0047edf](https://github.com/fluencelabs/rust-peer/commit/0047edfa60a06345aebbb5c5ed53de793963b08b))


### Bug Fixes

* **deps:** update rust crate fluence-keypair to 0.10.1 ([#1609](https://github.com/fluencelabs/rust-peer/issues/1609)) ([f5e0807](https://github.com/fluencelabs/rust-peer/commit/f5e080721ab43d154776dc4548b1280c2b089fb5))
* **identify:** fix address logging ([#1619](https://github.com/fluencelabs/rust-peer/issues/1619)) ([f90fdbf](https://github.com/fluencelabs/rust-peer/commit/f90fdbf5babfbd0cad8a8811c9869e07e9ad072e))
* **network:** fixed allow private addrs ([#1618](https://github.com/fluencelabs/rust-peer/issues/1618)) ([7efb490](https://github.com/fluencelabs/rust-peer/commit/7efb490af05693dcc4b46f9152b11359a4c384ef))

## [0.11.4](https://github.com/fluencelabs/rust-peer/compare/rust-peer-v0.11.3...rust-peer-v0.11.4) (2023-05-06)


### Bug Fixes

* aqua-pool-size argument ([96fdf54](https://github.com/fluencelabs/rust-peer/commit/96fdf54ba0b4c50ec8904d3f7ffbda15ab6d3772))
* **config:** aqua-pool-size argument ([#1606](https://github.com/fluencelabs/rust-peer/issues/1606)) ([96fdf54](https://github.com/fluencelabs/rust-peer/commit/96fdf54ba0b4c50ec8904d3f7ffbda15ab6d3772))
* **config:** correctly parse env variables ([#1607](https://github.com/fluencelabs/rust-peer/issues/1607)) ([2f77e04](https://github.com/fluencelabs/rust-peer/commit/2f77e040cd7c9ba3839ace5b5d4a2a6be9bcf929))
* **startup:** Change link in "hello" message to Discord ([#1604](https://github.com/fluencelabs/rust-peer/issues/1604)) ([c3c8b23](https://github.com/fluencelabs/rust-peer/commit/c3c8b2377efe3dffcd250778e3d407bac2bede39))

## [0.11.3](https://github.com/fluencelabs/rust-peer/compare/rust-peer-v0.11.2...rust-peer-v0.11.3) (2023-04-28)


### Bug Fixes

* **spells:** start resubscribed spells scheduling after full node initialization [fixes NET-459] ([#1592](https://github.com/fluencelabs/rust-peer/issues/1592)) ([1016f36](https://github.com/fluencelabs/rust-peer/commit/1016f368cbd6e9804908842ee2562bff0e241e0e))

## [0.11.2](https://github.com/fluencelabs/rust-peer/compare/rust-peer-v0.11.1...rust-peer-v0.11.2) (2023-04-27)


### Features

* **logging:** add structured fields in logging + new log format ([#1590](https://github.com/fluencelabs/rust-peer/issues/1590)) ([0c06f1b](https://github.com/fluencelabs/rust-peer/commit/0c06f1bd1e56b86b490a489b0b45e7dcb336f1d5))
* **logging:** structured logging and tracing ([#1582](https://github.com/fluencelabs/rust-peer/issues/1582)) ([7e97e45](https://github.com/fluencelabs/rust-peer/commit/7e97e4551236514c4bcb6a27cc3216bb817a2ab8))


### Bug Fixes

* **deps:** update rust crate fluence-spell-distro to v0.5.11 ([#1580](https://github.com/fluencelabs/rust-peer/issues/1580)) ([cb4e69e](https://github.com/fluencelabs/rust-peer/commit/cb4e69eebdb9617ed8f0083cd8c5c911039e99e1))
* **deps:** update rust crate fluence-spell-dtos to v0.5.11 ([#1581](https://github.com/fluencelabs/rust-peer/issues/1581)) ([2bd135e](https://github.com/fluencelabs/rust-peer/commit/2bd135e3c354d708f8c216c350a8e558d8243968))
* **metrics:** provide separate memory metrics for spells ([#1591](https://github.com/fluencelabs/rust-peer/issues/1591)) ([9034e4a](https://github.com/fluencelabs/rust-peer/commit/9034e4adfe065d5ae179e2831be05a0cda57eaaf))

## [0.11.1](https://github.com/fluencelabs/rust-peer/compare/rust-peer-v0.11.0...rust-peer-v0.11.1) (2023-04-20)


### Features

* add service type to distinguish services and spells ([#1567](https://github.com/fluencelabs/rust-peer/issues/1567)) ([d80f89b](https://github.com/fluencelabs/rust-peer/commit/d80f89bc8617a07b7cb3bb4103f1a015b924946e))
* **metrics:** add additional spell metrics [fixes NET-437] ([#1569](https://github.com/fluencelabs/rust-peer/issues/1569)) ([ab851ac](https://github.com/fluencelabs/rust-peer/commit/ab851accde1dfaca92b4011add1774caaa2bf9ea))
* **spells:** resolve spell_id by 'spell' and 'self' [NET-419] ([#1578](https://github.com/fluencelabs/rust-peer/issues/1578)) ([65b2b26](https://github.com/fluencelabs/rust-peer/commit/65b2b269c1355463c612d37f260d81c4422a793e))


### Bug Fixes

* long arguments [fixes NET-442] ([#1579](https://github.com/fluencelabs/rust-peer/issues/1579)) ([e4fd400](https://github.com/fluencelabs/rust-peer/commit/e4fd400cede9fb2d5c8df14324588a55520d4195))
* unnecessary vault creation ([#1566](https://github.com/fluencelabs/rust-peer/issues/1566)) ([a9be656](https://github.com/fluencelabs/rust-peer/commit/a9be6562a37ac40c5150dafd3b219a72486ef631))

## [0.11.0](https://github.com/fluencelabs/rust-peer/compare/rust-peer-v0.10.0...rust-peer-v0.11.0) (2023-04-12)


### ⚠ BREAKING CHANGES

* **logs:** print only first address of a Contact ([#1555](https://github.com/fluencelabs/rust-peer/issues/1555))
* **vault:** do not create vault on every particle ([#1553](https://github.com/fluencelabs/rust-peer/issues/1553))

### Features

* add service metrics for spells [fixes NET-441] ([#1565](https://github.com/fluencelabs/rust-peer/issues/1565)) ([947dfa8](https://github.com/fluencelabs/rust-peer/commit/947dfa8bfb75b438e79014c8585a4963c377c20b))
* **config:** Layered configuration [fixes NET-424] ([#1551](https://github.com/fluencelabs/rust-peer/issues/1551)) ([8e20c10](https://github.com/fluencelabs/rust-peer/commit/8e20c10705a7ff817d6a0f54909ef582415c7775))
* **logs:** print only first address of a Contact ([#1555](https://github.com/fluencelabs/rust-peer/issues/1555)) ([32a46b2](https://github.com/fluencelabs/rust-peer/commit/32a46b225a7ad8245c17908271cfc52739e6b989))
* **vault:** do not create vault on every particle ([#1553](https://github.com/fluencelabs/rust-peer/issues/1553)) ([aa0dd94](https://github.com/fluencelabs/rust-peer/commit/aa0dd94b87d17d9238d8d23736184bd74f95c1a1))


### Bug Fixes

* **deps:** update rust crate fluence-spell-distro to v0.5.9 ([#1559](https://github.com/fluencelabs/rust-peer/issues/1559)) ([47f5e20](https://github.com/fluencelabs/rust-peer/commit/47f5e206a2c5212ed87806b21f857feb4cdafc4e))
* **deps:** update rust crate fluence-spell-dtos to v0.5.9 ([#1560](https://github.com/fluencelabs/rust-peer/issues/1560)) ([f762b8b](https://github.com/fluencelabs/rust-peer/commit/f762b8b2f9eaa4c008dd931e3df651436a4ff2cb))
* **metrics:** Change _ to - in rust_peer_build_info ([#1558](https://github.com/fluencelabs/rust-peer/issues/1558)) ([c741e24](https://github.com/fluencelabs/rust-peer/commit/c741e24e826c74722167259b3fc578495c860586))
* **metrics:** restrict metrics collection of aliased service on creation ([#1557](https://github.com/fluencelabs/rust-peer/issues/1557)) ([040eb50](https://github.com/fluencelabs/rust-peer/commit/040eb505fce48d48e38a075d9bf50558a542467d))

## [0.10.0](https://github.com/fluencelabs/rust-peer/compare/rust-peer-v0.9.1...rust-peer-v0.10.0) (2023-04-06)


### ⚠ BREAKING CHANGES

* **builtins:** add worker.list, move spell.list to worker scope [fixes NET-401 NET-435] ([#1537](https://github.com/fluencelabs/rust-peer/issues/1537))
* **logs:** reduce logs ([#1534](https://github.com/fluencelabs/rust-peer/issues/1534))

### Features

* add restriction for installing modules with forbidden mounted binaries [NET-428] ([#1535](https://github.com/fluencelabs/rust-peer/issues/1535)) ([481dbd4](https://github.com/fluencelabs/rust-peer/commit/481dbd46c9eda18d22ec590514c71f9bfbb257d3))
* **builtins:** add worker.list, move spell.list to worker scope [fixes NET-401 NET-435] ([#1537](https://github.com/fluencelabs/rust-peer/issues/1537)) ([3884374](https://github.com/fluencelabs/rust-peer/commit/38843745fd028cecde275f3170053f62fcce06dc))
* collect metrics for spell particles separately [fixes NET-439] ([#1550](https://github.com/fluencelabs/rust-peer/issues/1550)) ([5711171](https://github.com/fluencelabs/rust-peer/commit/57111712bc88882f5d4510fa9cebe6868f87725b))
* extend peer identify with spell service version and allowed mounted binaries list [fixes NET-429 NET-381] ([#1540](https://github.com/fluencelabs/rust-peer/issues/1540)) ([30eff87](https://github.com/fluencelabs/rust-peer/commit/30eff87e1f9e20b01d7ffebb481dd820085775a7))
* **logs:** reduce logs ([#1534](https://github.com/fluencelabs/rust-peer/issues/1534)) ([dc69146](https://github.com/fluencelabs/rust-peer/commit/dc69146d6d096c2453450efad7e08d768ba78439))
* **metrics:** add rust-peer version info in prometheus metrics [fixes NET-422] ([#1552](https://github.com/fluencelabs/rust-peer/issues/1552)) ([460446b](https://github.com/fluencelabs/rust-peer/commit/460446b01bab79117a4d61c6e40965d3e72a9945))


### Bug Fixes

* bug with repeated alias for service [NET-434] ([#1536](https://github.com/fluencelabs/rust-peer/issues/1536)) ([ff455be](https://github.com/fluencelabs/rust-peer/commit/ff455be54132a5b08977d91ef5bf67e6264d2c80))
* collect separate metrics for root alised services and worker spells [fixes NET-431] ([#1539](https://github.com/fluencelabs/rust-peer/issues/1539)) ([270cfd2](https://github.com/fluencelabs/rust-peer/commit/270cfd2c3a1236fc2cbb19b68b0bb4714e871148))
* **deps:** update rust crate air-interpreter-wasm to v0.38.0 ([#1502](https://github.com/fluencelabs/rust-peer/issues/1502)) ([12c74fb](https://github.com/fluencelabs/rust-peer/commit/12c74fbe8cbedcc701838d153d4d8e85af80e979))
* **deps:** update rust crate fluence-app-service to 0.25.1 ([#1522](https://github.com/fluencelabs/rust-peer/issues/1522)) ([446c342](https://github.com/fluencelabs/rust-peer/commit/446c3422ec8c386f2f5d1eb0a62b69245e04863b))
* **deps:** update rust crate fluence-app-service to 0.25.3 ([#1541](https://github.com/fluencelabs/rust-peer/issues/1541)) ([e5ecf30](https://github.com/fluencelabs/rust-peer/commit/e5ecf30ad79bcfa70b214f3a07f94507153270f4))
* **deps:** update rust crate fluence-it-types to 0.4.1 ([#1523](https://github.com/fluencelabs/rust-peer/issues/1523)) ([0480d06](https://github.com/fluencelabs/rust-peer/commit/0480d06ea7a187d0511f4198a0ccb53edf4c5bfc))
* **deps:** update rust crate fluence-spell-distro to v0.5.7 ([#1532](https://github.com/fluencelabs/rust-peer/issues/1532)) ([d027160](https://github.com/fluencelabs/rust-peer/commit/d027160ad8341d4bac9b22e3dc18630488307141))
* **deps:** update rust crate fluence-spell-dtos to v0.5.7 ([#1533](https://github.com/fluencelabs/rust-peer/issues/1533)) ([804cab8](https://github.com/fluencelabs/rust-peer/commit/804cab8a0ea2ef513671f3a27ec9d543ad9ce5db))
* don't create libp2p metrics twice [fixes NET-348] ([#1545](https://github.com/fluencelabs/rust-peer/issues/1545)) ([d023865](https://github.com/fluencelabs/rust-peer/commit/d0238657e6c0e23df1ac14efea3b25e2efb270ac))
* **metrics:** collect metrics for custom services [fixes NET-438] ([#1549](https://github.com/fluencelabs/rust-peer/issues/1549)) ([ed1ce37](https://github.com/fluencelabs/rust-peer/commit/ed1ce37b393f8aa94f31f40ea5bff59194eadf7c))
* **spells:** update trigger config by alias [NET-418] ([#1521](https://github.com/fluencelabs/rust-peer/issues/1521)) ([0531848](https://github.com/fluencelabs/rust-peer/commit/053184859054c3cc2aa684a5c67d74a7125d87f7))

## [0.9.1](https://github.com/fluencelabs/rust-peer/compare/rust-peer-v0.9.0...rust-peer-v0.9.1) (2023-03-21)


### Bug Fixes

* **ci:** fix tokio build for publish ([#1518](https://github.com/fluencelabs/rust-peer/issues/1518)) ([8e7efac](https://github.com/fluencelabs/rust-peer/commit/8e7efac096d2ec36e71056301d7e7b012a0015c3))
* **deps:** update rust crate fluence-spell-dtos to v0.5.6 ([#1517](https://github.com/fluencelabs/rust-peer/issues/1517)) ([d93bece](https://github.com/fluencelabs/rust-peer/commit/d93bece432cc6962b607ee99d83713f62ca572d1))

## [0.9.0](https://github.com/fluencelabs/rust-peer/compare/rust-peer-v0.8.2...rust-peer-v0.9.0) (2023-03-21)


### ⚠ BREAKING CHANGES

* **marine,avm:** avm-server 0.31.0, fluence-app-service 0.25.0 ([#1506](https://github.com/fluencelabs/rust-peer/issues/1506))
* **deps:** update fluence-app-service minor version ([#1497](https://github.com/fluencelabs/rust-peer/issues/1497))

### Features

* **async runtime:** move async-std to tokio ([#1469](https://github.com/fluencelabs/rust-peer/issues/1469)) ([08615a2](https://github.com/fluencelabs/rust-peer/commit/08615a25711e7584d42ca2e97449e7812be5e332))
* **deps:** update fluence-app-service minor version ([#1497](https://github.com/fluencelabs/rust-peer/issues/1497)) ([8c82741](https://github.com/fluencelabs/rust-peer/commit/8c82741468083d38daaf73fd3a59be1f90fc2d2d))
* **marine,avm:** avm-server 0.31.0, fluence-app-service 0.25.0 ([#1506](https://github.com/fluencelabs/rust-peer/issues/1506)) ([23820e9](https://github.com/fluencelabs/rust-peer/commit/23820e93e8c5a5749f976f63520544fa88c621a2))
* **worker:** add worker.remove [fixes NET-354 NET-376] ([#1499](https://github.com/fluencelabs/rust-peer/issues/1499)) ([97f552f](https://github.com/fluencelabs/rust-peer/commit/97f552f4fdbd94d943bd4896f5e8491ab791cd0e))


### Bug Fixes

* **deps:** update rust crate fluence-it-types to 0.4.0 ([#1467](https://github.com/fluencelabs/rust-peer/issues/1467)) ([34cfc85](https://github.com/fluencelabs/rust-peer/commit/34cfc853a3dd1ac57d6806491e1b074690be62f0))
* **deps:** update rust crate fluence-spell-distro to v0.5.6 ([#1516](https://github.com/fluencelabs/rust-peer/issues/1516)) ([97ceb92](https://github.com/fluencelabs/rust-peer/commit/97ceb9202d0e665dc29f728bf4ce60e474e7f5c2))
* **deps:** update rust crate fluence-spell-dtos to v0.5.4 ([#1471](https://github.com/fluencelabs/rust-peer/issues/1471)) ([b7c6b4d](https://github.com/fluencelabs/rust-peer/commit/b7c6b4d206330d4c8d05932f533d43c1c078f1ca))

## [0.8.2](https://github.com/fluencelabs/rust-peer/compare/rust-peer-v0.8.1...rust-peer-v0.8.2) (2023-02-27)


### Bug Fixes

* **avm-server:** update avm-server to one that user particle_id and peer_id in data_store ([#1494](https://github.com/fluencelabs/rust-peer/issues/1494)) ([5ede2b0](https://github.com/fluencelabs/rust-peer/commit/5ede2b031bc20fc7cb1220727cbc6c424610bafa))

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
