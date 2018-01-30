import SbtCommons._

name := "fluence"

scalacOptions in Compile ++= Seq("-Ypartial-unification", "-Xdisable-assertions")

javaOptions in Test ++= Seq("-ea")

commons

enablePlugins(AutomateHeaderPlugin)

lazy val `codec-core` = project.in(file("codec/core"))

lazy val `codec-kryo` = project.in(file("codec/kryo"))
  .dependsOn(`codec-core`)

lazy val `kademlia-node` = project.in(file("kademlia/node"))
  .dependsOn(`kademlia-protocol`)

lazy val `kademlia-protocol` = project.in(file("kademlia/protocol"))
  .dependsOn(`codec-core`, `crypto`)

lazy val `kademlia-testkit` = project.in(file("kademlia/testkit"))
  .dependsOn(`kademlia-node`)

lazy val `kademlia-grpc` = project.in(file("kademlia/grpc"))
  .dependsOn(`transport-grpc`, `kademlia-protocol`, `codec-core`, `kademlia-testkit` % Test)

lazy val `transport-grpc` = project.in(file("transport/grpc"))
  .dependsOn(`transport-core`, `codec-core`)

lazy val `transport-core` = project.in(file("transport/core"))
  .dependsOn(`kademlia-protocol`)

lazy val `storage` = project.in(file("storage/core"))
  .dependsOn(`codec-core`)

lazy val `storage-rocksdb` = project.in(file("storage/rocksdb"))
  .dependsOn(`storage`)

lazy val `b-tree-client` = project.in(file("b-tree/client"))
  .dependsOn(`b-tree-common`, `b-tree-protocol`)

lazy val `b-tree-common` = project.in(file("b-tree/common"))
  .dependsOn(`crypto`)

lazy val `b-tree-protocol` = project.in(file("b-tree/protocol"))
  .dependsOn(`b-tree-common`)

lazy val `b-tree-server` = project.in(file("b-tree/server"))
  .dependsOn(`storage-rocksdb`, `codec-kryo`, `b-tree-common`, `b-tree-protocol`, `b-tree-client` % "compile->test")

lazy val `crypto` = project.in(file("crypto"))

lazy val `dataset-node` = project.in(file("dataset/node"))
  .dependsOn(`storage`, `kademlia-node`, `b-tree-server`, `kademlia-testkit` % Test, `dataset-client`, `b-tree-client`,
              `dataset-client` % "compile->test")

lazy val `dataset-protocol` = project.in(file("dataset/protocol"))
  .dependsOn(`kademlia-protocol`, `b-tree-protocol`)

lazy val `dataset-grpc` = project.in(file("dataset/grpc"))
  .dependsOn(`dataset-client`, `transport-grpc`)

lazy val `dataset-client` = project.in(file("dataset/client"))
  .dependsOn(`dataset-protocol`, `crypto`, `b-tree-client`)

lazy val `info-protocol` = project.in(file("info/protocol"))
lazy val `info-grpc` = project.in(file("info/grpc")).dependsOn(`info-protocol`, `transport-grpc`)
lazy val `info-node` = project.in(file("info/node")).dependsOn(`info-protocol`)

lazy val `node` = project
  .dependsOn(`kademlia-grpc`, `kademlia-node`, `dataset-node`, `dataset-grpc`, `info-grpc`, `info-node`, `client`)

// TODO: grpc is only for JVM: transport should be more abstract
lazy val `client` = project.in(file("client"))
  .dependsOn(`dataset-client`, `transport-grpc`, `kademlia-grpc`, `dataset-grpc`, `info-grpc`)