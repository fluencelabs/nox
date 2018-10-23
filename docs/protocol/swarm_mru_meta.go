package protocol

type SwarmMeta struct {
  Key        string  // resource key
  Version    int     // resource version
  Content    []byte  // uploaded content
  Permission Seal    // owner signature which authorizes the update
}

func SwarmMRUMetaExample() {
  // data
  var swarmMRU map[string]map[int]SwarmMeta  // MRU storage: key –> version –> content

  {
    // just to make things compile
    _ = swarmMRU
  }
}
