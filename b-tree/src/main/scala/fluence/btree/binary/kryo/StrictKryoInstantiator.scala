package fluence.btree.binary.kryo

import com.twitter.chill.{ AllScalaRegistrar, KryoBase, KryoInstantiator }
import org.objenesis.strategy.StdInstantiatorStrategy

/**
 * This Instantiator enable compulsory class registration, registers all java and scala main classes.
 * This class required for [[com.twitter.chill.KryoPool]].
 * @param classesToReg additional classes for registration
 */
case class StrictKryoInstantiator(classesToReg: Seq[Class[_]]) extends KryoInstantiator {

  override def newKryo(): KryoBase = {
    val kryo = new KryoBase()
    kryo.setRegistrationRequired(true)
    kryo.setInstantiatorStrategy(new StdInstantiatorStrategy())
    new AllScalaRegistrar()(kryo)
    // in future will be able to create specific fast serializer for each class
    kryo.registerClasses(classesToReg)
    kryo
  }
}
