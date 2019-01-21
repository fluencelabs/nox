/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.node
import java.io.File
import java.net.InetAddress

import cats.syntax.functor._
import org.scalatest.{Timer => _}

import scala.language.higherKinds
import scala.sys.process.{Process, ProcessLogger}

trait OsSetup {
  protected val bootstrapDir = new File("../bootstrap")
  protected def runCmd(cmd: String): Unit = Process(cmd, bootstrapDir).!(ProcessLogger(_ => ()))
  protected def runBackground(cmd: String): Unit = Process(cmd, bootstrapDir).run(ProcessLogger(_ => ()))
  protected val dockerHost: String = getOS match {
    case "linux" => ifaceIP("docker0")
    case "mac" => "host.docker.internal"
    case os => throw new RuntimeException(s"$os isn't supported")
  }

  protected val ethereumHost: String = getOS match {
    case "linux" => linuxHostIP.get
    case "mac" => "host.docker.internal"
    case os => throw new RuntimeException(s"$os isn't supported")
  }

  // return IP address of the `interface`
  protected def ifaceIP(interface: String): String = {
    import sys.process._
    val ifconfigCmd = Seq("ifconfig", interface)
    val grepCmd = Seq("grep", "inet ")
    val awkCmd = Seq("awk", "{print $2}")
    InetAddress.getByName((ifconfigCmd #| grepCmd #| awkCmd).!!.replaceAll("[^0-9\\.]", "")).getHostAddress
  }

  protected def linuxHostIP = {
    import sys.process._
    val ipR = "(?<=src )[0-9\\.]+".r
    ipR.findFirstIn("ip route get 8.8.8.8".!!.trim)
  }

  protected def getOS: String = {
    // TODO: should use more comprehensive and reliable OS detection
    val osName = System.getProperty("os.name").toLowerCase()
    if (osName.contains("windows"))
      "windows"
    else if (osName.contains("mac") || osName.contains("darwin"))
      "mac"
    else
      "linux"
  }
}
