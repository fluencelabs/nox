addSbtPlugin("com.geirsson" % "sbt-scalafmt" % "1.4.0")

addSbtPlugin("com.typesafe.sbt" % "sbt-native-packager" % "1.3.2")

addSbtPlugin("de.heikoseeberger" % "sbt-header" % "4.1.0")

addSbtPlugin("com.thesamet" % "sbt-protoc" % "0.99.18")

addSbtPlugin("org.scala-js"       % "sbt-scalajs"              % "0.6.22")
addSbtPlugin("org.portable-scala" % "sbt-crossproject"         % "0.3.1")
addSbtPlugin("org.portable-scala" % "sbt-scalajs-crossproject" % "0.3.1")

addSbtPlugin("ch.epfl.scala" % "sbt-scalajs-bundler" % "0.10.0")

addSbtPlugin("net.virtual-void" % "sbt-dependency-graph" % "0.9.0")

addSbtPlugin("com.lihaoyi" % "workbench" % "0.4.0")

addSbtPlugin("ch.epfl.scala" % "sbt-bloop" % "1.0.0-M11")

libraryDependencies += "com.thesamet.scalapb" %% "compilerplugin" % "0.7.2"
