package fluence.ethclient.data

sealed case class Genesis(genesis_time: String, chain_id: String, app_hash: String, validators: Array[Validator])

sealed case class Validator(pub_key: String, power: String, name: String)
