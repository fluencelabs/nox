/*
 * Copyright (C) 2018  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.swarm

/**
 * Parameters for the address when requesting the download of mutable resources.
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
