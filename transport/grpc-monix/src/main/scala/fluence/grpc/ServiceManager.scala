/*
 * Copyright (C) 2017  Fluence Labs Limited
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

package fluence.grpc

import cats.MonadError
import cats.syntax.flatMap._
import io.grpc._

import scala.collection.JavaConverters._
import scala.language.higherKinds

class ServiceManager(services: Map[String, MethodDescriptor[Any, Any]]) {

  /**
   * Gets grpc method descriptor from registered services.
   *
   * @param service Name of service.
   * @param method Name of method.
   *
   * @return Method descriptor or None, if there is no descriptor in registered services.
   */
  private def getMethodDescriptor(service: String, method: String): Option[MethodDescriptor[Any, Any]] = {
    for {
      methodDescriptor ← services.get(service + "/" + method)
    } yield methodDescriptor
  }

  def getMethodDescriptorF[F[_]](service: String, method: String)(
    implicit F: MonadError[F, Throwable]
  ): F[MethodDescriptor[Any, Any]] =
    F.pure(getMethodDescriptor(service, method)).flatMap {
      case Some(md) ⇒ F.pure(md)
      case None ⇒ F.raiseError(new IllegalArgumentException(s"There is no $service/$method method."))
    }

}

object ServiceManager {

  def apply(server: Server): ServiceManager = {
    val methodDescriptorMap: Map[String, MethodDescriptor[Any, Any]] = server.getServices.asScala
      .flatMap(_.getMethods.asScala.map(_.getMethodDescriptor.asInstanceOf[MethodDescriptor[Any, Any]]))
      .map(m ⇒ m.getFullMethodName -> m)
      .toMap
    new ServiceManager(methodDescriptorMap)
  }

  def apply(serverDescriptors: List[ServiceDescriptor]): ServiceManager = {
    val methodDescriptorMap = serverDescriptors
      .flatMap(sd ⇒ sd.getMethods.asScala.map(_.asInstanceOf[MethodDescriptor[Any, Any]]))
      .map(m ⇒ m.getFullMethodName -> m)
      .toMap

    new ServiceManager(methodDescriptorMap)
  }
}
