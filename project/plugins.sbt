addSbtPlugin("org.scalariform" % "sbt-scalariform" % "1.8.2")

addSbtPlugin("com.typesafe.sbt" % "sbt-native-packager" % "1.3.2")

addSbtPlugin("de.heikoseeberger" % "sbt-header" % "4.1.0")

addSbtPlugin("com.thesamet" % "sbt-protoc" % "0.99.14")

addSbtPlugin("org.scala-js" % "sbt-scalajs" % "0.6.22")
addSbtPlugin("org.portable-scala" % "sbt-crossproject"         % "0.3.1")
addSbtPlugin("org.portable-scala" % "sbt-scalajs-crossproject" % "0.3.1")

addSbtPlugin("ch.epfl.scala" % "sbt-scalajs-bundler" % "0.10.0")

addSbtPlugin("net.virtual-void" % "sbt-dependency-graph" % "0.9.0")

libraryDependencies += "com.trueaccord.scalapb" %% "compilerplugin" % "0.6.6"
// For preventing classpath conflict in tests, which sometimes leads to errors: 'parameter object not a ECParameterSpec'
// Cause 'org.bouncycastle.jce.spec.ECNamedCurveParameterSpec' is not a subtype of 'ECParameterSpec' in bouncycastle version 1.52
libraryDependencies += "org.bouncycastle" % "bcprov-jdk15on" % "1.59" force()