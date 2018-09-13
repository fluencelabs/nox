## External Storage

External storage is a part of [Fluence project](https://github.com/fluencelabs/fluence) 
that is responsible for storage the history of operations, developer's code and another information 
that should be highly available in Fluence network and from the World Wide Web.

Current implementation represents API in Scala for interaction with [Swarm](https://swarm-guide.readthedocs.io/en/latest/introduction.html).
Swarm was selected after some research. We found it more complete, well-documented and with effective incentivization model.

## Documentation

### Installation

Before API usage you need to install [Swarm](https://swarm-guide.readthedocs.io/en/latest/installation.html) and [Ethereum client](https://ethereum.github.io/go-ethereum/install/)

### Before started


**create Ethereum account, if you don't have one** 

```geth account new```

**start Ethereum client (possible in light mode, if you don't want to download full node)**

```text
geth --syncmode "light"
```

**start Swarm client with ENS support**
```text
swarm --bzzaccount ETH_ACCOUNT --ens-api /path/to/.ethereum/geth.ipc
```

### Uploading and downloading

Let's create an API client with the address of Swarm:
```scala
import SwarmClient._
val api = SwarmClient("localhost", 8500)
```
It is possible to use Swarm gateway, if you want.

After that we can upload something:

```scala
val hash = api.uploadUnsafe(Array[Byte](1,2,3))

// it can be any hash, because of Swarm returns hash of manifest
println(hash)

```
And then download it:
```scala
val result = api.downloadUnsafe(hash)

// 1, 2, 3
println(result.mkString(", "))
```

In type-safe way you can do it like this:

```scala
for {
  hash <- api.upload(Array[Byte](1,2,3))
  result <- api.download(hash)
} yield {
  println(hash)
  println(result.mkString(", "))
}
```

### Mutable Resource Updates (MRU)

The private key is required to work with the MRU to verify ownership of the content.

As a key you can use generated key for Ethereum:

```scala
import java.math.BigInteger
import org.web3j.crypto.ECKeyPair

val secret = new BigInteger(1, ByteVector.fromValidHex("PRIVATE_KEY_HEX").toArray)

val key: ECKeyPair = ECKeyPair.create(secret) 
``` 

or generate a new one:

```scala
import org.web3j.crypto.Keys

val key: ECKeyPair = Keys.createEcKeyPair()
```

Your ethAddress will be:

```scala
val ethAddress = Keys.getAddress(key)
```

And method for signing:
```scala
val signer = Secp256k1Signer.signer(key)
```

You can create a Mutable Resource after those preparations. For this we will generate identifier of Mutable Resource:
```scala
import scala.concurrent.duration._

val id = MutableResourceIdentifier(
    name = Some("name of resource"), // optional resource name. You can use any name
    frequency = 300 seconds, // expected time interval between updates, in seconds
    startTime = System.currentTimeMillis() millis, // time the resource is valid from, in Unix time (seconds)
    ownerAddr = ethAddress)
```

and then create the resource in Swarm with some data, update it and get the first and second version. 
Let's just do it in a type-safe way: 
```scala

val data = ByteVector("Some random string in Swarm!".getBytes)

for {
    hashOfMR <- api.initializeMutableResource(id, data, false, signer)
    _ = println("Hash of Mutable Resource: " + hashOfMR)
    
    dataVersion2 = ByteVector("Now is end!".getBytes)
    period = 1
    version = 2
    
    _ <- api.updateMutableResource(id, dataVersion2, false, period, version, signer)

    result1 <- api.downloadMutableResource(hashOfMR)
    _ = println("Last version of mutable resource: " + new String(result1.toArray))

    result2 <- api.downloadMutableResource(hashOfMR, Some(Period(1, Some(2))))
    _ = println("Another last version of mutable resource: " + new String(result1.toArray))

    result3 <- api.downloadMutableResource(hashOfMR, Some(Period(1, Some(1))))
    _ = println("First version: " + new String(result3.toArray))
  } yield ()
```

## Project status
The project is undergoing a heavy development at the moment.  
Check out the [issue tracker](https://github.com/fluencelabs/fluence/issues) to learn more about the current progress.

## Contributing
You are welcome to contribute. At the current moment we don't have detailed instructions on how to join development or which code guidelines to follow. However, you can expect more info to appear soon enough. In the meanwhile, check out the [basic contributing rules](./CONTRIBUTING.md).

## License
[Apache 2.0](./LICENSE.md)