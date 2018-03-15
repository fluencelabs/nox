import sbt._
import scalajscrossproject.ScalaJSCrossPlugin.autoImport._
import sbtcrossproject.CrossPlugin.autoImport._

/**
 * cross types https://github.com/portable-scala/sbt-crossproject
 * http://xuwei-k.github.io/slides/scala-js-matsuri/#21
 * avoid move files
 */
object FluenceCrossType extends sbtcrossproject.CrossType {
  override def projectDir(crossBase: File, projectType: String) =
    crossBase / projectType

  override def projectDir(crossBase: File, projectType: sbtcrossproject.Platform) = {
    val dir = projectType match {
      case JVMPlatform ⇒ "jvm"
      case JSPlatform  ⇒ "js"
    }
    crossBase / dir
  }

  def shared(projectBase: File, conf: String) =
    projectBase.getParentFile / "src" / conf / "scala"

  override def sharedSrcDir(projectBase: File, conf: String) =
    Some(shared(projectBase, conf))
}
