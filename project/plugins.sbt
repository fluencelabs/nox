addSbtPlugin("org.scalariform" % "sbt-scalariform" % "1.8.2")

addSbtPlugin("com.typesafe.sbt" % "sbt-native-packager" % "1.3.2")

addSbtPlugin("de.heikoseeberger" % "sbt-header" % "4.1.0")

addSbtPlugin("com.thesamet" % "sbt-protoc" % "0.99.14")

addSbtPlugin("org.scala-js" % "sbt-scalajs" % "0.6.22")
addSbtPlugin("org.portable-scala" % "sbt-crossproject"         % "0.3.1")
addSbtPlugin("org.portable-scala" % "sbt-scalajs-crossproject" % "0.3.1")

libraryDependencies += "com.trueaccord.scalapb" %% "compilerplugin" % "0.6.6"