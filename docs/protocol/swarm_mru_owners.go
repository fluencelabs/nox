package protocol

type SwarmOwners struct  {
  data map[string]PublicKey  // owners: resource key –> owner's public key
}

// initializes and returns a new resource key
func (owners *SwarmOwners) Init(permission Seal) string { panic("") }

// returns an owner's public key for the specified resource key
func (owners *SwarmOwners) Owner(resourceKey string) PublicKey { panic("") }

func SwarmMRUOwnersExample() {
  // data
  var swarmOwners SwarmOwners                // list of resource owners for each key
  var swarmMRU map[string]map[int]SwarmMeta  // MRU storage: key –> version –> content

  // rules
  var key     string                         // some key
  var version int                            // some version

  // ∀ key, ∀ version:
  // the content stored for specific key and version should be properly authorized by its owner
  var meta = swarmMRU[key][version]

  assert(meta.Key == key)
  assert(meta.Version == version)
  assert(meta.Permission.PublicKey == swarmOwners.Owner(meta.Key))
  assert(SwarmVerify(meta.Permission, SwarmHash(pack(meta.Key, meta.Version, meta.Content))))
}
