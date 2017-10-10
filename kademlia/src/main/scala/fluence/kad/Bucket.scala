package fluence.kad

import cats.{Applicative, MonadError, Show}
import cats.data.StateT
import cats.syntax.eq._
import cats.syntax.flatMap._
import cats.syntax.applicativeError._
import cats.syntax.applicative._

import scala.language.higherKinds

case class Bucket(maxSize: Int, contacts: Seq[Contact] = List.empty) {
  lazy val isFull: Boolean = contacts.size >= maxSize

  lazy val size: Int = contacts.size

  def isEmpty: Boolean = contacts.isEmpty

  def nonEmpty: Boolean = contacts.nonEmpty
}

object Bucket {
  implicit def show(implicit cs: Show[Contact]): Show[Bucket] =
    b => if (b.contacts.isEmpty) "[empty bucket]" else b.contacts.map(cs.show).mkString(s"[${b.size} of ${b.maxSize}\n\t", "\n\t", "]")

  /**
    * Returns the bucket state
    *
    * @tparam F StateT effect
    * @return The bucket
    */
  def bucket[F[_] : Applicative]: StateT[F, Bucket, Bucket] = StateT.get[F, Bucket]

  /**
    * Lookups the bucket for a particular key, not making any external request
    *
    * @param key Contact key
    * @tparam F StateT effect
    * @return optional found contact
    */
  def find[F[_] : Applicative](key: Key): StateT[F, Bucket, Option[Contact]] =
    bucket[F].map { b =>
      b.contacts.find(_.key === key)
    }

  /**
    * Performs bucket update.
    *
    * Whenever a node receives a communication from another, it updates the corresponding bucket.
    * If the contact already exists, it is _moved_ to the (head) of the bucket.
    * Otherwise, if the bucket is not full, the new contact is _added_ at the (head).
    * If the bucket is full, the node pings the contact at the (end) of the bucket's list.
    * If that least recently seen contact fails to respond in an (unspecified) reasonable time,
    * it is _dropped_ from the list, and the new contact is added at the (head).
    * Otherwise the new contact is _ignored_ for bucket updating purposes.
    *
    * @param contact Contact to check and update
    * @param ping    Ping function
    * @param ME      Monad error for StateT effect
    * @tparam F StateT effect
    * @return updated Bucket
    */
  def update[F[_]](contact: Contact, ping: Contact => F[Contact])(implicit ME: MonadError[F, Throwable]): StateT[F, Bucket, Unit] = {
    bucket[F].flatMap { b =>
      find[F](contact.key).flatMap {
        case Some(c) =>
          // put contact on top
          StateT set b.copy(contacts = contact +: b.contacts.filterNot(_.key == c.key))
        case None if b.isFull =>
          // ping last, if pong, put last on top and drop contact, if not, drop last and put contact on top
          StateT setF ping(b.contacts.last).attempt.flatMap {
            case Left(_) =>
              b.copy(contacts = contact +: b.contacts.dropRight(1)).pure
            case Right(updatedLastContact) =>
              b.copy(contacts = updatedLastContact +: b.contacts.dropRight(1)).pure
          }
        case None =>
          // put contact on top
          StateT set b.copy(contacts = contact +: b.contacts)
      }
    }
  }

}