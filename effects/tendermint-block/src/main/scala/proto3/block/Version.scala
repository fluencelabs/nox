// Generated by the Scala Plugin for the Protocol Buffer Compiler.
// Do not edit!
//
// Protofile syntax: PROTO3

package proto3.block

@SerialVersionUID(0L)
final case class Version(
    block: _root_.scala.Long = 0L,
    app: _root_.scala.Long = 0L
    ) extends scalapb.GeneratedMessage with scalapb.Message[Version] with scalapb.lenses.Updatable[Version] {
    @transient
    private[this] var __serializedSizeCachedValue: _root_.scala.Int = 0
    private[this] def __computeSerializedValue(): _root_.scala.Int = {
      var __size = 0
      
      {
        val __value = block
        if (__value != 0L) {
          __size += _root_.com.google.protobuf.CodedOutputStream.computeUInt64Size(1, __value)
        }
      };
      
      {
        val __value = app
        if (__value != 0L) {
          __size += _root_.com.google.protobuf.CodedOutputStream.computeUInt64Size(2, __value)
        }
      };
      __size
    }
    final override def serializedSize: _root_.scala.Int = {
      var read = __serializedSizeCachedValue
      if (read == 0) {
        read = __computeSerializedValue()
        __serializedSizeCachedValue = read
      }
      read
    }
    def writeTo(`_output__`: _root_.com.google.protobuf.CodedOutputStream): _root_.scala.Unit = {
      {
        val __v = block
        if (__v != 0L) {
          _output__.writeUInt64(1, __v)
        }
      };
      {
        val __v = app
        if (__v != 0L) {
          _output__.writeUInt64(2, __v)
        }
      };
    }
    def mergeFrom(`_input__`: _root_.com.google.protobuf.CodedInputStream): proto3.block.Version = {
      var __block = this.block
      var __app = this.app
      var _done__ = false
      while (!_done__) {
        val _tag__ = _input__.readTag()
        _tag__ match {
          case 0 => _done__ = true
          case 8 =>
            __block = _input__.readUInt64()
          case 16 =>
            __app = _input__.readUInt64()
          case tag => _input__.skipField(tag)
        }
      }
      proto3.block.Version(
          block = __block,
          app = __app
      )
    }
    def withBlock(__v: _root_.scala.Long): Version = copy(block = __v)
    def withApp(__v: _root_.scala.Long): Version = copy(app = __v)
    def getFieldByNumber(__fieldNumber: _root_.scala.Int): _root_.scala.Any = {
      (__fieldNumber: @_root_.scala.unchecked) match {
        case 1 => {
          val __t = block
          if (__t != 0L) __t else null
        }
        case 2 => {
          val __t = app
          if (__t != 0L) __t else null
        }
      }
    }
    def getField(__field: _root_.scalapb.descriptors.FieldDescriptor): _root_.scalapb.descriptors.PValue = {
      _root_.scala.Predef.require(__field.containingMessage eq companion.scalaDescriptor)
      (__field.number: @_root_.scala.unchecked) match {
        case 1 => _root_.scalapb.descriptors.PLong(block)
        case 2 => _root_.scalapb.descriptors.PLong(app)
      }
    }
    def toProtoString: _root_.scala.Predef.String = _root_.scalapb.TextFormat.printToUnicodeString(this)
    def companion = proto3.block.Version
}

object Version extends scalapb.GeneratedMessageCompanion[proto3.block.Version] {
  implicit def messageCompanion: scalapb.GeneratedMessageCompanion[proto3.block.Version] = this
  def fromFieldsMap(__fieldsMap: scala.collection.immutable.Map[_root_.com.google.protobuf.Descriptors.FieldDescriptor, _root_.scala.Any]): proto3.block.Version = {
    _root_.scala.Predef.require(__fieldsMap.keys.forall(_.getContainingType() == javaDescriptor), "FieldDescriptor does not match message type.")
    val __fields = javaDescriptor.getFields
    proto3.block.Version(
      __fieldsMap.getOrElse(__fields.get(0), 0L).asInstanceOf[_root_.scala.Long],
      __fieldsMap.getOrElse(__fields.get(1), 0L).asInstanceOf[_root_.scala.Long]
    )
  }
  implicit def messageReads: _root_.scalapb.descriptors.Reads[proto3.block.Version] = _root_.scalapb.descriptors.Reads{
    case _root_.scalapb.descriptors.PMessage(__fieldsMap) =>
      _root_.scala.Predef.require(__fieldsMap.keys.forall(_.containingMessage == scalaDescriptor), "FieldDescriptor does not match message type.")
      proto3.block.Version(
        __fieldsMap.get(scalaDescriptor.findFieldByNumber(1).get).map(_.as[_root_.scala.Long]).getOrElse(0L),
        __fieldsMap.get(scalaDescriptor.findFieldByNumber(2).get).map(_.as[_root_.scala.Long]).getOrElse(0L)
      )
    case _ => throw new RuntimeException("Expected PMessage")
  }
  def javaDescriptor: _root_.com.google.protobuf.Descriptors.Descriptor = BlockProto.javaDescriptor.getMessageTypes.get(3)
  def scalaDescriptor: _root_.scalapb.descriptors.Descriptor = BlockProto.scalaDescriptor.messages(3)
  def messageCompanionForFieldNumber(__number: _root_.scala.Int): _root_.scalapb.GeneratedMessageCompanion[_] = throw new MatchError(__number)
  lazy val nestedMessagesCompanions: Seq[_root_.scalapb.GeneratedMessageCompanion[_]] = Seq.empty
  def enumCompanionForFieldNumber(__fieldNumber: _root_.scala.Int): _root_.scalapb.GeneratedEnumCompanion[_] = throw new MatchError(__fieldNumber)
  lazy val defaultInstance = proto3.block.Version(
  )
  implicit class VersionLens[UpperPB](_l: _root_.scalapb.lenses.Lens[UpperPB, proto3.block.Version]) extends _root_.scalapb.lenses.ObjectLens[UpperPB, proto3.block.Version](_l) {
    def block: _root_.scalapb.lenses.Lens[UpperPB, _root_.scala.Long] = field(_.block)((c_, f_) => c_.copy(block = f_))
    def app: _root_.scalapb.lenses.Lens[UpperPB, _root_.scala.Long] = field(_.app)((c_, f_) => c_.copy(app = f_))
  }
  final val BLOCK_FIELD_NUMBER = 1
  final val APP_FIELD_NUMBER = 2
}
