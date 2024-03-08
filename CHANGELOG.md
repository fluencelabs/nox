# Changelog

## [0.22.2](https://github.com/fluencelabs/nox/compare/nox-v0.22.1...nox-v0.22.2) (2024-03-08)


### Features

* **workers:** separate AVM pools [fixes NET-753] ([#2125](https://github.com/fluencelabs/nox/issues/2125)) ([fd2552f](https://github.com/fluencelabs/nox/commit/fd2552f94a61c106d7c03b8b2bb18eaf47eae8a1))


### Bug Fixes

* **config:** add humantime & default for proof_poll_period ([#2139](https://github.com/fluencelabs/nox/issues/2139)) ([fd6acf6](https://github.com/fluencelabs/nox/commit/fd6acf65c211deec1be84a4bd4333a3b68f05f8f))
* **config:** change default for the cpu range in nox ([#2140](https://github.com/fluencelabs/nox/issues/2140)) ([c976373](https://github.com/fluencelabs/nox/commit/c976373c6e25171580f995e98635535eae597b87))

## [0.22.1](https://github.com/fluencelabs/nox/compare/nox-v0.22.0...nox-v0.22.1) (2024-03-07)


### Bug Fixes

* **deps:** update rust crate decider-distro to v0.6.11 ([#2137](https://github.com/fluencelabs/nox/issues/2137)) ([bd8ff99](https://github.com/fluencelabs/nox/commit/bd8ff996ac5d6449844e99031b234fa9ef3cce76))
* **workers:** add io+timer drivers to tokio runtimes [fixes NET-795] ([#2135](https://github.com/fluencelabs/nox/issues/2135)) ([fb251c9](https://github.com/fluencelabs/nox/commit/fb251c92a244d360e3fd1a46edcb00221cb4c75a))

## [0.22.0](https://github.com/fluencelabs/nox/compare/nox-v0.21.4...nox-v0.22.0) (2024-03-05)


### ⚠ BREAKING CHANGES

* **aquavm:** aquavm mem limits from config [fixes VM-425] ([#2111](https://github.com/fluencelabs/nox/issues/2111))

### Features

* **aquavm:** aquavm mem limits from config [fixes VM-425] ([#2111](https://github.com/fluencelabs/nox/issues/2111)) ([a732f5c](https://github.com/fluencelabs/nox/commit/a732f5ca16e2cd5a721a220ce8f73b784dd0ab21))

## [0.21.4](https://github.com/fluencelabs/nox/compare/nox-v0.21.3...nox-v0.21.4) (2024-03-03)


### Features

* bump fluence-app-service ([#2130](https://github.com/fluencelabs/nox/issues/2130)) ([98161b1](https://github.com/fluencelabs/nox/commit/98161b1d22ae4d62383c946c5ae415f5e9c5b9ed))

## [0.21.3](https://github.com/fluencelabs/nox/compare/nox-v0.21.2...nox-v0.21.3) (2024-03-01)


### Features

* introduce the dev mode in which alow all effectors  ([#2127](https://github.com/fluencelabs/nox/issues/2127)) ([0eee6bf](https://github.com/fluencelabs/nox/commit/0eee6bf536cb5dc49c81421dc235dafcf17820c6))

## [0.21.2](https://github.com/fluencelabs/nox/compare/nox-v0.21.1...nox-v0.21.2) (2024-02-27)


### Features

* introduce DealId type ([#2124](https://github.com/fluencelabs/nox/issues/2124)) ([ed0984d](https://github.com/fluencelabs/nox/commit/ed0984d62ffec7dfb0c6c5010900e4bad363010f))
* restrict effectors ([#2110](https://github.com/fluencelabs/nox/issues/2110)) ([a4485ab](https://github.com/fluencelabs/nox/commit/a4485ab3a36f091941b5d4cc06b952fdac6c702e))

## [0.21.1](https://github.com/fluencelabs/nox/compare/nox-v0.21.0...nox-v0.21.1) (2024-02-24)


### Bug Fixes

* **deps:** update decider 0.6.9 ([#2120](https://github.com/fluencelabs/nox/issues/2120)) ([e8ca595](https://github.com/fluencelabs/nox/commit/e8ca5958b2a7dfcc625443227ef0856c199ab3cd))

## [0.21.0](https://github.com/fluencelabs/nox/compare/nox-v0.20.2...nox-v0.21.0) (2024-02-23)


### ⚠ BREAKING CHANGES

* **particle-vault:** introduce new particle vault path format ([#2098](https://github.com/fluencelabs/nox/issues/2098))

### Features

* add connector.send_tx; introduce listener ([#2078](https://github.com/fluencelabs/nox/issues/2078)) ([81f00ea](https://github.com/fluencelabs/nox/commit/81f00eacb7c2337f9f7b6a8f4313b8da88a26339))
* **config:** Introduce ephemeral and persistent storage [fixes NET-759 NET-760] ([#2091](https://github.com/fluencelabs/nox/issues/2091)) ([79e7850](https://github.com/fluencelabs/nox/commit/79e7850be08429afdfb284af91c9e42b95ab6336))
* **particle-vault:** introduce new particle vault path format ([#2098](https://github.com/fluencelabs/nox/issues/2098)) ([3d68c85](https://github.com/fluencelabs/nox/commit/3d68c857ac804f798b74649d2468df72df3a06f0))
* **spell:** update to new kv restrictions ([#2094](https://github.com/fluencelabs/nox/issues/2094)) ([7bc9e5a](https://github.com/fluencelabs/nox/commit/7bc9e5acb3390a8f8dec96e7628e9f042044cc1c))


### Bug Fixes

* **core-manager:** add logging info ([#2109](https://github.com/fluencelabs/nox/issues/2109)) ([06f253b](https://github.com/fluencelabs/nox/commit/06f253b7e44cf655701ca2def4cb9d2ea08eba2b))
* **deps:** update fluence-spell to v0.7.3 ([#2088](https://github.com/fluencelabs/nox/issues/2088)) ([77c6e86](https://github.com/fluencelabs/nox/commit/77c6e86ad2e2cba2b2274bf9a4929811ebbcc068))
* **deps:** update rust crate decider-distro to v0.6.6 ([#2104](https://github.com/fluencelabs/nox/issues/2104)) ([938f751](https://github.com/fluencelabs/nox/commit/938f75150e7f9d53cd267ff0f35ee37303584cdd))

## [0.20.2](https://github.com/fluencelabs/nox/compare/nox-v0.20.1...nox-v0.20.2) (2024-02-21)


### Features

* support new CallParameters and generate particle token [NET-767] ([179d697](https://github.com/fluencelabs/nox/commit/179d6974f916b8c1b059f1277c062e3272e45714))


### Bug Fixes

* **core-manager:** process core persistence in a stream ([#2100](https://github.com/fluencelabs/nox/issues/2100)) ([7f333bd](https://github.com/fluencelabs/nox/commit/7f333bd9bc1d6d343fd3ff1cda4d9b08e11205f3))
* **deps:** decider 0.6.4, with debug logs for tx ([#2101](https://github.com/fluencelabs/nox/issues/2101)) ([23e556b](https://github.com/fluencelabs/nox/commit/23e556b5d79d50460ff375d3824a843c0c512e8f))

## [0.20.1](https://github.com/fluencelabs/nox/compare/nox-v0.20.0...nox-v0.20.1) (2024-02-21)


### Bug Fixes

* **deps:** decider distro 0.6.1 ([#2096](https://github.com/fluencelabs/nox/issues/2096)) ([b07e524](https://github.com/fluencelabs/nox/commit/b07e524f84ab7c34c673a90757cb85fefb9f5074))
* **deps:** update rust crate decider-distro to v0.6.3 ([#2089](https://github.com/fluencelabs/nox/issues/2089)) ([61aa65c](https://github.com/fluencelabs/nox/commit/61aa65cf5e3a786d663a988b531870829f2f0fa6))
* **deps:** use toml 0.5.10 ([#2095](https://github.com/fluencelabs/nox/issues/2095)) ([df662c5](https://github.com/fluencelabs/nox/commit/df662c5f7162a583e3b649cc8c280340968aa230))
* **deps:** use toml 0.7 in server-config ([#2097](https://github.com/fluencelabs/nox/issues/2097)) ([a4221cb](https://github.com/fluencelabs/nox/commit/a4221cb70b6f4cf88b9df8eb7c1427e24c42ffc5))
* **particle-vault:** refactor particle vault usage ([#2077](https://github.com/fluencelabs/nox/issues/2077)) ([26f998d](https://github.com/fluencelabs/nox/commit/26f998d2b72732ed7eab8338f15daecd2f7339a6))

## [0.20.0](https://github.com/fluencelabs/nox/compare/nox-v0.19.0...nox-v0.20.0) (2024-02-19)


### ⚠ BREAKING CHANGES

* **cores:** core manager [fixes NET-739  NET-755 NET-740] ([#2069](https://github.com/fluencelabs/nox/issues/2069))

### Features

* **app-services:** Support worker-id in CallParameters [NET-717] ([#2039](https://github.com/fluencelabs/nox/issues/2039)) ([3b1c10f](https://github.com/fluencelabs/nox/commit/3b1c10f3996c684c4f3143dfa2eca87adf5a9642))
* **cores:** core manager [fixes NET-739  NET-755 NET-740] ([#2069](https://github.com/fluencelabs/nox/issues/2069)) ([7366f84](https://github.com/fluencelabs/nox/commit/7366f840bb6774670adc1aa09a9eb596dc337362))


### Bug Fixes

* **deps:** update rust crate decider-distro to v0.5.17 ([#2081](https://github.com/fluencelabs/nox/issues/2081)) ([046417e](https://github.com/fluencelabs/nox/commit/046417ed835e2e71e4fb3beb20b205cc454b5925))
* **subnet-resolve:** update subnet resolve to the new ComputeUnit structure ([#2027](https://github.com/fluencelabs/nox/issues/2027)) ([7d561b7](https://github.com/fluencelabs/nox/commit/7d561b77d2e23bf1b6c5c0bd016cbdd66e8eb337))

## [0.19.0](https://github.com/fluencelabs/nox/compare/nox-v0.18.1...nox-v0.19.0) (2024-02-15)


### ⚠ BREAKING CHANGES

* **worker_pool:** Shift service creation to the worker pool [Fixes NET-716] ([#2026](https://github.com/fluencelabs/nox/issues/2026))

### Features

* **created-swarm:** Re-export some dependencies in created-swarm for consistency in tests ([#2079](https://github.com/fluencelabs/nox/issues/2079)) ([8f0c04e](https://github.com/fluencelabs/nox/commit/8f0c04e10baa2e7597a433778e7ee471bbfab750))
* **health:** add kademlia bootstrap healthcheck + fix test span logging ([#2056](https://github.com/fluencelabs/nox/issues/2056)) ([c082231](https://github.com/fluencelabs/nox/commit/c0822317fc532e3f1ce9f0b25a625208c503b829))
* **worker_pool:** Shift service creation to the worker pool [Fixes NET-716] ([#2026](https://github.com/fluencelabs/nox/issues/2026)) ([d3e4855](https://github.com/fluencelabs/nox/commit/d3e4855671f747ea9815420ebf38d83262fd687d))


### Bug Fixes

* **deps:** update rust crate decider-distro to v0.5.16 ([#2053](https://github.com/fluencelabs/nox/issues/2053)) ([247f577](https://github.com/fluencelabs/nox/commit/247f5773d58262aeb0d6fe7e4f27dfa4371a7098))
* **particle-data-store:** use signature in particle data store path name [NET-632] ([#2052](https://github.com/fluencelabs/nox/issues/2052)) ([3065a95](https://github.com/fluencelabs/nox/commit/3065a950246d8a10116095aac64b06d4814e4bd4))

## [0.18.1](https://github.com/fluencelabs/nox/compare/nox-v0.18.0...nox-v0.18.1) (2024-01-31)


### Bug Fixes

* **deps:** update fluence-spell to v0.6.10 ([#2049](https://github.com/fluencelabs/nox/issues/2049)) ([5dc200e](https://github.com/fluencelabs/nox/commit/5dc200e59fa16c7abd3c3cc9c9063eca76cb51cc))

## [0.18.0](https://github.com/fluencelabs/nox/compare/nox-v0.17.0...nox-v0.18.0) (2024-01-31)


### ⚠ BREAKING CHANGES

* **particle-data:** use MsgPack for particle data serialization [Fixes VM-407] ([#2034](https://github.com/fluencelabs/nox/issues/2034))

### Features

* **anomaly:** new anomaly criteria [fixes vm-426] ([#2037](https://github.com/fluencelabs/nox/issues/2037)) ([d59b885](https://github.com/fluencelabs/nox/commit/d59b88560610d0641bdd7bd75e438dce238dd772))
* **particle-data:** use MsgPack for particle data serialization [Fixes VM-407] ([#2034](https://github.com/fluencelabs/nox/issues/2034)) ([4d76e8f](https://github.com/fluencelabs/nox/commit/4d76e8f3bedb18e3406b7d223c56eb74541a7f97))

## [0.17.0](https://github.com/fluencelabs/nox/compare/nox-v0.16.15...nox-v0.17.0) (2024-01-26)


### ⚠ BREAKING CHANGES

* **deps:** avm server 0.35 aquavm 0.59 ([#2040](https://github.com/fluencelabs/nox/issues/2040))

### Bug Fixes

* **deps:** avm server 0.35 aquavm 0.59 ([#2040](https://github.com/fluencelabs/nox/issues/2040)) ([bb7227e](https://github.com/fluencelabs/nox/commit/bb7227ebea2915f19f40f8a628b1b5826bbee446))

## [0.16.15](https://github.com/fluencelabs/nox/compare/nox-v0.16.14...nox-v0.16.15) (2024-01-23)


### Features

* **services,avm:** use marine and avm-server with memory limits ([#1957](https://github.com/fluencelabs/nox/issues/1957)) ([66edf0f](https://github.com/fluencelabs/nox/commit/66edf0f005976db6e1fe73f98b00c8f7b4d8d6c7))
* **workers:** Create a ThreadPool for each worker [fixes NET-688] ([#2012](https://github.com/fluencelabs/nox/issues/2012)) ([fa5a17a](https://github.com/fluencelabs/nox/commit/fa5a17a0ce68a7270625f8487517df130bb387ba))


### Bug Fixes

* **deps:** update fluence-spell to v0.6.8 ([#2013](https://github.com/fluencelabs/nox/issues/2013)) ([5ea8615](https://github.com/fluencelabs/nox/commit/5ea86158720742cbc3949b45f2e2e19cb0fd4959))
* **deps:** update fluence-spell to v0.6.9 ([#2024](https://github.com/fluencelabs/nox/issues/2024)) ([725b6b4](https://github.com/fluencelabs/nox/commit/725b6b4648c6ef4f02f5958ca97429dffd128c39))
* **deps:** update rust crate aqua-ipfs-distro to v0.5.30 ([#2011](https://github.com/fluencelabs/nox/issues/2011)) ([15e33c4](https://github.com/fluencelabs/nox/commit/15e33c4b75df468598541d0da1487c5155938ed8))
* **deps:** update rust crate decider-distro to v0.5.14 ([#2035](https://github.com/fluencelabs/nox/issues/2035)) ([9a6823f](https://github.com/fluencelabs/nox/commit/9a6823f6267b67cff63be2df71a73a6e15a70922))
* **deps:** update rust crate trust-graph-distro to v0.4.11 ([#2025](https://github.com/fluencelabs/nox/issues/2025)) ([e2bbef7](https://github.com/fluencelabs/nox/commit/e2bbef7c4c445a9d82444e67669e791230e6dd20))

## [0.16.14](https://github.com/fluencelabs/nox/compare/nox-v0.16.13...nox-v0.16.14) (2024-01-12)


### Features

* introduce chain-listener [fixes NET-694 NET-673 NET-674 NET-677 NET-685] ([#1972](https://github.com/fluencelabs/nox/issues/1972)) ([3ff44b5](https://github.com/fluencelabs/nox/commit/3ff44b52d9709c370ade267bd8d4f30c1f376735))
* **worker:** Refactoring key manager [fixes NET-702] ([#1985](https://github.com/fluencelabs/nox/issues/1985)) ([345f9b8](https://github.com/fluencelabs/nox/commit/345f9b89617dfe1a2eb399de27dc5ba36e5c8d1d))


### Bug Fixes

* **deps:** update rust crate decider-distro to v0.5.13 ([#2016](https://github.com/fluencelabs/nox/issues/2016)) ([0ea3a85](https://github.com/fluencelabs/nox/commit/0ea3a85cd0c1801a99467040aeddc6ab3fed56bb))
* **deps:** update rust crate registry-distro to v0.9.4 ([#2010](https://github.com/fluencelabs/nox/issues/2010)) ([3f8c176](https://github.com/fluencelabs/nox/commit/3f8c176121bea8b547933163f84f145f1f5f9389))
* **deps:** update rust crate trust-graph-distro to v0.4.10 ([#2008](https://github.com/fluencelabs/nox/issues/2008)) ([70798eb](https://github.com/fluencelabs/nox/commit/70798ebaf488aa236fda0d48594c94f50dd1cb81))

## [0.16.13](https://github.com/fluencelabs/nox/compare/nox-v0.16.12...nox-v0.16.13) (2023-12-29)


### Bug Fixes

* **config:** respect base_dir in keypair generation ([#1998](https://github.com/fluencelabs/nox/issues/1998)) ([e2b99bf](https://github.com/fluencelabs/nox/commit/e2b99bff5917197f6b02d5a415a60bdd7b14a98b))
* **deps:** update rust crate decider-distro to v0.5.12 ([#2000](https://github.com/fluencelabs/nox/issues/2000)) ([c35e6e3](https://github.com/fluencelabs/nox/commit/c35e6e39aeaf153aeff413ea8be67e03efbd1252))
* **deps:** update rust crate fluence-keypair to 0.10.4 ([#1990](https://github.com/fluencelabs/nox/issues/1990)) ([ed5f5d0](https://github.com/fluencelabs/nox/commit/ed5f5d0dafc175a9a81bd33bbe8b891e6a0be62e))
* **deps:** update rust crate registry-distro to v0.9.2 ([#1970](https://github.com/fluencelabs/nox/issues/1970)) ([fb072a0](https://github.com/fluencelabs/nox/commit/fb072a0198f0d9c961739952723ffe37fc976ec2))
* **deps:** update rust crate trust-graph-distro to v0.4.9 ([#1999](https://github.com/fluencelabs/nox/issues/1999)) ([7b9b376](https://github.com/fluencelabs/nox/commit/7b9b37685426e0e766fa46dadbdb3f6c3bdaacd0))

## [0.16.12](https://github.com/fluencelabs/nox/compare/nox-v0.16.11...nox-v0.16.12) (2023-12-28)


### Bug Fixes

* **deps:** update fluence-spell to v0.6.4 ([#1991](https://github.com/fluencelabs/nox/issues/1991)) ([8c9bfb9](https://github.com/fluencelabs/nox/commit/8c9bfb93d6e9c77be8982040178323a4862b7ba2))
* **deps:** update rust crate aqua-ipfs-distro to v0.5.28 ([#1992](https://github.com/fluencelabs/nox/issues/1992)) ([eb89451](https://github.com/fluencelabs/nox/commit/eb8945183846d8a8346c0583da815995baa52b63))
* **deps:** update rust crate decider-distro to v0.5.11 ([#1993](https://github.com/fluencelabs/nox/issues/1993)) ([bb0fa32](https://github.com/fluencelabs/nox/commit/bb0fa329e522566e84decc44e9758e9a54942887))
* **deps:** update rust crate trust-graph-distro to v0.4.8 ([#1994](https://github.com/fluencelabs/nox/issues/1994)) ([94cf9fa](https://github.com/fluencelabs/nox/commit/94cf9fac24f246458b987a2ba53088fddad29631))

## [0.16.11](https://github.com/fluencelabs/nox/compare/nox-v0.16.10...nox-v0.16.11) (2023-12-26)


### Bug Fixes

* **deps:** update fluence-spell to v0.6.3 ([#1988](https://github.com/fluencelabs/nox/issues/1988)) ([5a1f865](https://github.com/fluencelabs/nox/commit/5a1f865ab8603b01f7f4febcd22dd599210c3e2b))

## [0.16.10](https://github.com/fluencelabs/nox/compare/nox-v0.16.9...nox-v0.16.10) (2023-12-26)


### Features

* **metrics:** add libp2p bandwidth metrics ([#1973](https://github.com/fluencelabs/nox/issues/1973)) ([090dff3](https://github.com/fluencelabs/nox/commit/090dff3450cf4336b25aae297332069548f14618))


### Bug Fixes

* **deps:** update fluence-spell to v0.6.2 ([#1986](https://github.com/fluencelabs/nox/issues/1986)) ([47ecbe0](https://github.com/fluencelabs/nox/commit/47ecbe0969c007eaab75abc60885427e67ad2f90))
* **deps:** update rust crate aqua-ipfs-distro to v0.5.27 ([#1987](https://github.com/fluencelabs/nox/issues/1987)) ([67f7240](https://github.com/fluencelabs/nox/commit/67f7240ba94dd746e94efdf4e21bd6ff19524139))
* **deps:** update rust crate decider-distro to v0.5.9 ([#1975](https://github.com/fluencelabs/nox/issues/1975)) ([d5d47e3](https://github.com/fluencelabs/nox/commit/d5d47e37f7a62a97e5bc74a6cedd61dd39fe22a9))

## [0.16.9](https://github.com/fluencelabs/nox/compare/nox-v0.16.8...nox-v0.16.9) (2023-12-20)


### Features

* **plumber:** allow root to access deactivated workers [fixes NET-695] ([#1968](https://github.com/fluencelabs/nox/issues/1968)) ([2d13c8a](https://github.com/fluencelabs/nox/commit/2d13c8aa1a74b102143ae2b7aa2d82eaa805fad7))


### Bug Fixes

* **cleanup:** cleanup par + max batch size ([#1967](https://github.com/fluencelabs/nox/issues/1967)) ([1063eae](https://github.com/fluencelabs/nox/commit/1063eae0211bb4803db29c75a104bdd5bce0121a))
* **deps:** update fluence-spell to v0.6.0 ([#1958](https://github.com/fluencelabs/nox/issues/1958)) ([5bc6495](https://github.com/fluencelabs/nox/commit/5bc64951c1914d7d6eaf523742aa4eda3c43a5c5))

## [0.16.8](https://github.com/fluencelabs/nox/compare/nox-v0.16.7...nox-v0.16.8) (2023-12-18)


### Bug Fixes

* **datastore:** remove particles ([#1965](https://github.com/fluencelabs/nox/issues/1965)) ([17e5e2e](https://github.com/fluencelabs/nox/commit/17e5e2eaf95bd2362326f5a3b32923cd3008575e))

## [0.16.7](https://github.com/fluencelabs/nox/compare/nox-v0.16.6...nox-v0.16.7) (2023-12-15)


### Features

* **tracing:** Add tracing for understand particle processing ([#1935](https://github.com/fluencelabs/nox/issues/1935)) ([c800495](https://github.com/fluencelabs/nox/commit/c800495f23d59a343239f1d3ce57beee6e8b51ac))


### Bug Fixes

* **datastore:** remove directories ([#1956](https://github.com/fluencelabs/nox/issues/1956)) ([3fa5321](https://github.com/fluencelabs/nox/commit/3fa53213b1e5e83256f4700685ed8ea179f478fe))
* **spells:** allow empty args for callbackSrv [NET-651] ([#1942](https://github.com/fluencelabs/nox/issues/1942)) ([19e00be](https://github.com/fluencelabs/nox/commit/19e00bed75a6d5db5072157a1ddcd159aa8e42bf))

## [0.16.6](https://github.com/fluencelabs/nox/compare/nox-v0.16.5...nox-v0.16.6) (2023-12-13)


### Features

* **avm:** Use AVMRunner instead of AVM ([#1952](https://github.com/fluencelabs/nox/issues/1952)) ([fe3d563](https://github.com/fluencelabs/nox/commit/fe3d5632080856d19bdefecbd88441673be5cf6a))


### Bug Fixes

* **deps:** update rust crate air-interpreter-wasm to 0.55.0 ([#1953](https://github.com/fluencelabs/nox/issues/1953)) ([1dfc777](https://github.com/fluencelabs/nox/commit/1dfc7771c8373b4efc100864687b0556a871daeb))

## [0.16.5](https://github.com/fluencelabs/nox/compare/nox-v0.16.4...nox-v0.16.5) (2023-12-08)


### Bug Fixes

* **deps:** bump decider-distro to 0.5.6 ([0e7d24f](https://github.com/fluencelabs/nox/commit/0e7d24ff465707707083213e42ea14f59511fe00))

## [0.16.4](https://github.com/fluencelabs/nox/compare/nox-v0.16.3...nox-v0.16.4) (2023-12-01)


### Features

* **workers:** add activate/deactivate [NET-587] ([#1889](https://github.com/fluencelabs/nox/issues/1889)) ([0883ab8](https://github.com/fluencelabs/nox/commit/0883ab8912ca19cb28dc9b6b388c48d5e6aa7306))


### Bug Fixes

* **deps:** update fluence-spell to v0.5.32 ([#1937](https://github.com/fluencelabs/nox/issues/1937)) ([5460b36](https://github.com/fluencelabs/nox/commit/5460b3697cc951297210a81373761ae4550bb31a))

## [0.16.3](https://github.com/fluencelabs/nox/compare/nox-v0.16.2...nox-v0.16.3) (2023-11-21)


### Features

* **config:** add multiple file config sources ([#1907](https://github.com/fluencelabs/nox/issues/1907)) ([8bba3ea](https://github.com/fluencelabs/nox/commit/8bba3eab395053ef7dedab464f31ca5ced9e03d2))

## [0.16.2](https://github.com/fluencelabs/nox/compare/nox-v0.16.1...nox-v0.16.2) (2023-11-16)


### Features

* update decider and aqua-ipfs distros ([#1910](https://github.com/fluencelabs/nox/issues/1910)) ([2238b4a](https://github.com/fluencelabs/nox/commit/2238b4a1b781ca36b56914a48c30b57cf27aece9))


### Bug Fixes

* **deps:** update fluence-spell to v0.5.31 ([#1905](https://github.com/fluencelabs/nox/issues/1905)) ([9cedae3](https://github.com/fluencelabs/nox/commit/9cedae3219e4b8fd691f1f82fa40ac887a8188f5))

## [0.16.1](https://github.com/fluencelabs/nox/compare/nox-v0.16.0...nox-v0.16.1) (2023-11-15)


### Features

* **metrics:** Add tokio runtime metrics ([#1878](https://github.com/fluencelabs/nox/issues/1878)) ([5522bed](https://github.com/fluencelabs/nox/commit/5522bed9164d8e9ee142e0410db45f61ada6323c))
* **nox:** add new flags for system services [NET-547, NET-548] ([#1888](https://github.com/fluencelabs/nox/issues/1888)) ([b172117](https://github.com/fluencelabs/nox/commit/b172117c869d06abc88485a1e5ddf0e98aadf8e4))
* remove insecured keypair [NET-633] ([#1906](https://github.com/fluencelabs/nox/issues/1906)) ([32e938c](https://github.com/fluencelabs/nox/commit/32e938c06a35f4267cb44db83635aadf81095404))
* **subnet:** Update resolver to new contracts [NET-619] ([#1891](https://github.com/fluencelabs/nox/issues/1891)) ([0de0556](https://github.com/fluencelabs/nox/commit/0de0556a6eae02d309449e71e8d3aaa1f15a7da4))
* **tracing:** add sample rate + filtering ([#1904](https://github.com/fluencelabs/nox/issues/1904)) ([e825dd1](https://github.com/fluencelabs/nox/commit/e825dd1e4df4b28e2f2034fad3b26de22f0c198f))
* use particle signature as id for actors [NET-540] ([#1877](https://github.com/fluencelabs/nox/issues/1877)) ([da84a39](https://github.com/fluencelabs/nox/commit/da84a39ffd514a6819e4392c4c7b1fc4bf2a9d35))


### Bug Fixes

* **deps:** update fluence-spell to v0.5.28 ([#1890](https://github.com/fluencelabs/nox/issues/1890)) ([291cb70](https://github.com/fluencelabs/nox/commit/291cb708902f0f09b25193fea0c86661a76baabf))
* **logging:** add a target field in the logfmt formatter ([1bcfb1c](https://github.com/fluencelabs/nox/commit/1bcfb1c37b09e98d4d18130cb3b5e9e60611b55c))
* **logging:** fixed logfmt formatter ([#1885](https://github.com/fluencelabs/nox/issues/1885)) ([1bcfb1c](https://github.com/fluencelabs/nox/commit/1bcfb1c37b09e98d4d18130cb3b5e9e60611b55c))
* **metrics:** separate a avm call time & interpretation time metrics ([#1893](https://github.com/fluencelabs/nox/issues/1893)) ([e94d1e4](https://github.com/fluencelabs/nox/commit/e94d1e4e7aae9c39a9b87f0e80a8484fcc2592d9))

## [0.16.0](https://github.com/fluencelabs/nox/compare/nox-v0.15.2...nox-v0.16.0) (2023-10-30)


### ⚠ BREAKING CHANGES

* **deps:** update avm

### Features

* **deps:** update avm ([cc7c535](https://github.com/fluencelabs/nox/commit/cc7c535f0a422db754e22a809d112b9ddc1b3940))


### Bug Fixes

* **metrics:** measure call time and wait time separately ([#1858](https://github.com/fluencelabs/nox/issues/1858)) ([73bab7e](https://github.com/fluencelabs/nox/commit/73bab7ebc76d623c3e8304e6e68925a9163a0ecb))
* **system-services:** refactor deployer + fix [NET-586] ([#1859](https://github.com/fluencelabs/nox/issues/1859)) ([78b0a46](https://github.com/fluencelabs/nox/commit/78b0a46241cec8dd4767f9f46d4e800b0a3a62a1))

## [0.15.2](https://github.com/fluencelabs/nox/compare/nox-v0.15.1...nox-v0.15.2) (2023-10-29)


### Bug Fixes

* **builtins:** validate deal_id in subnet.resolve [NET-591] ([#1842](https://github.com/fluencelabs/nox/issues/1842)) ([84bc025](https://github.com/fluencelabs/nox/commit/84bc025b8e36a5e6bef5e0d2fe2ad804241d7526))
* **deps:** Update avm packages in one PR and bump fluence-app-service ([#1839](https://github.com/fluencelabs/nox/issues/1839)) ([ce5bbfa](https://github.com/fluencelabs/nox/commit/ce5bbfa626e1a8154e2fd6fd7d0829d4fd5c6b0e))
* **deps:** update fluence-spell to v0.5.25 ([#1854](https://github.com/fluencelabs/nox/issues/1854)) ([ebe781f](https://github.com/fluencelabs/nox/commit/ebe781f82b12938cd699d9cf014a74d1b82914ac))
* **deps:** update fluence-spell to v0.5.26 ([#1855](https://github.com/fluencelabs/nox/issues/1855)) ([d1dbc88](https://github.com/fluencelabs/nox/commit/d1dbc88543bcf2f29cc8119b6d8c866594ee5cd0))
* **deps:** update rust crate decider-distro to v0.5.3 ([#1872](https://github.com/fluencelabs/nox/issues/1872)) ([40dcae2](https://github.com/fluencelabs/nox/commit/40dcae2e1304ed71a4c1684edafd1de2050e411e))
* remove aqua cli from README ([#1843](https://github.com/fluencelabs/nox/issues/1843)) ([8012b3e](https://github.com/fluencelabs/nox/commit/8012b3efddd454e5e27cf32a125efcb11c202d7a))

## [0.15.1](https://github.com/fluencelabs/nox/compare/nox-v0.15.0...nox-v0.15.1) (2023-10-16)


### Bug Fixes

* **deps:** update rust crate air-interpreter-wasm to 0.52.0 ([#1827](https://github.com/fluencelabs/nox/issues/1827)) ([608e0d6](https://github.com/fluencelabs/nox/commit/608e0d6ee94ec21c97c99b558fa86b7cc4d22b84))
* **deps:** update rust crate avm-server to 0.33.2 ([#1838](https://github.com/fluencelabs/nox/issues/1838)) ([96df6bb](https://github.com/fluencelabs/nox/commit/96df6bb29d1d39383bc60617b143ceba2b208439))

## [0.15.0](https://github.com/fluencelabs/nox/compare/rust-peer-v0.14.9...rust-peer-v0.15.0) (2023-10-11)


### ⚠ BREAKING CHANGES

* remove scheduled scripts [NET-566] ([#1812](https://github.com/fluencelabs/nox/issues/1812))

### Features

* remove scheduled scripts [NET-566] ([#1812](https://github.com/fluencelabs/nox/issues/1812)) ([cd9995a](https://github.com/fluencelabs/nox/commit/cd9995a6e2ca2336c23f7d4e117adc8215c74215))
* verify particle signatures [NET-539] ([#1811](https://github.com/fluencelabs/nox/issues/1811)) ([188d833](https://github.com/fluencelabs/nox/commit/188d833af07159921fa04864eaaf54a75eaa2fa1))


### Bug Fixes

* **deps:** update rust crate fluence-spell-distro to v0.5.22 ([#1795](https://github.com/fluencelabs/nox/issues/1795)) ([1df86e8](https://github.com/fluencelabs/nox/commit/1df86e8047bb4aab168ab6e5cfb5a978c573dfe0))
* **deps:** update rust crate fluence-spell-distro to v0.5.24 ([#1825](https://github.com/fluencelabs/nox/issues/1825)) ([abea305](https://github.com/fluencelabs/nox/commit/abea305cd221f0daafb5f4420fff9cc08747c6bf))
* **deps:** update rust crate fluence-spell-dtos to v0.5.22 ([#1796](https://github.com/fluencelabs/nox/issues/1796)) ([5c4ea1b](https://github.com/fluencelabs/nox/commit/5c4ea1b9adba5997989fc751d39bc0e5cbdcd7af))
* env configuration loading for protocol config ([#1814](https://github.com/fluencelabs/nox/issues/1814)) ([e8a11fe](https://github.com/fluencelabs/nox/commit/e8a11fee5a4d6364e01ecbba220cfd7dac47fe68))
* **spell-bus:** resubscribe spell on double subscription ([#1797](https://github.com/fluencelabs/nox/issues/1797)) ([7ee8efa](https://github.com/fluencelabs/nox/commit/7ee8efa4d321a027081c1e9736bc9a5415ae7f86))
* **tests:** Allow override system services config in tests [NET-576] ([#1824](https://github.com/fluencelabs/nox/issues/1824)) ([2c6b6e3](https://github.com/fluencelabs/nox/commit/2c6b6e3fe321ec2be875427f927329eb8ac4a2b6))

## [0.14.9](https://github.com/fluencelabs/nox/compare/rust-peer-v0.14.8...rust-peer-v0.14.9) (2023-09-22)


### Features

* **decider:** allow to set ipfs multiaddr from env [fixes NET-544] ([#1802](https://github.com/fluencelabs/nox/issues/1802)) ([a0606bc](https://github.com/fluencelabs/nox/commit/a0606bc069051b592106dec1929508781bce253d))


### Bug Fixes

* **deps:** update rust crate avm-server to 0.33.1 ([#1799](https://github.com/fluencelabs/nox/issues/1799)) ([4dfa59d](https://github.com/fluencelabs/nox/commit/4dfa59d761a71b0c2f63466584cd207af5354886))

## [0.14.8](https://github.com/fluencelabs/nox/compare/rust-peer-v0.14.7...rust-peer-v0.14.8) (2023-09-22)


### Bug Fixes

* **deps:** update rust crate air-interpreter-wasm to 0.48.0 ([#1800](https://github.com/fluencelabs/nox/issues/1800)) ([31bd096](https://github.com/fluencelabs/nox/commit/31bd09684ca25c18b014d2b576ee6210f0ca3bdc))

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
