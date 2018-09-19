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

package fluence.swarm

/**
 * Parameters for the address when requesting the download of mutable resources.
 * Select between the download concrete period and the version or download only the metafile.
 * @see [[SwarmClient.downloadMutableResource()]]
 */
sealed trait DownloadResourceParam {
  def toParams: Seq[String]
}

/**
 * We can directly specify the period and version when requesting the download of mutable resources.
 *
 * @param period Indicates for what period we want to download. Depending on the current time, startTime and frequency.
 * @param version Indicates what resource version of the period we want to download.
 */
case class Period(period: Int, version: Option[Int]) extends DownloadResourceParam {
  override def toParams: Seq[String] = (period +: version.toSeq).map(_.toString)
}

/**
 * Parameter that indicates, that we want to download only meta information of a mutable resource (period, version,
 * frequency, startTime, name, rootAddr, etc).
 */
case object Meta extends DownloadResourceParam {
  override def toParams: scala.Seq[String] = Seq("meta")
}
