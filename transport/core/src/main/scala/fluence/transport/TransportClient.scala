package fluence.transport

import fluence.kad.protocol.Contact
import shapeless.{ HList, ops }

trait TransportClient[CL <: HList] {
  /**
   * Returns a service stub for a particular contact.
   *
   * @param contact To open service for
   * @param sel     Implicit selector from HList
   * @tparam T Type of the service
   * @return
   */
  def service[T](contact: Contact)(implicit sel: ops.hlist.Selector[CL, T]): T
}
