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

// Generated by the Scala Plugin for the Protocol Buffer Compiler.
// Do not edit!
//
// Protofile syntax: PROTO3

package proto3.tendermint

@SerialVersionUID(0L)
final case class PartSetHeader(
  total: _root_.scala.Int = 0,
  hash: _root_.com.google.protobuf.ByteString = _root_.com.google.protobuf.ByteString.EMPTY
) extends scalapb.GeneratedMessage with scalapb.Message[PartSetHeader] with scalapb.lenses.Updatable[PartSetHeader] {

  @transient
  private[this] var __serializedSizeCachedValue: _root_.scala.Int = 0
  private[this] def __computeSerializedValue(): _root_.scala.Int = {
    var __size = 0

    {
      val __value = total
      if (__value != 0) {
        __size += _root_.com.google.protobuf.CodedOutputStream.computeInt32Size(1, __value)
      }
    };

    {
      val __value = hash
      if (__value != _root_.com.google.protobuf.ByteString.EMPTY) {
        __size += _root_.com.google.protobuf.CodedOutputStream.computeBytesSize(2, __value)
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
      val __v = total
      if (__v != 0) {
        _output__.writeInt32(1, __v)
      }
    };
    {
      val __v = hash
      if (__v != _root_.com.google.protobuf.ByteString.EMPTY) {
        _output__.writeBytes(2, __v)
      }
    };
  }

  def mergeFrom(`_input__`: _root_.com.google.protobuf.CodedInputStream): proto3.tendermint.PartSetHeader = {
    var __total = this.total
    var __hash = this.hash
    var _done__ = false
    while (!_done__) {
      val _tag__ = _input__.readTag()
      _tag__ match {
        case 0 => _done__ = true
        case 8 =>
          __total = _input__.readInt32()
        case 18 =>
          __hash = _input__.readBytes()
        case tag => _input__.skipField(tag)
      }
    }
    proto3.tendermint.PartSetHeader(
      total = __total,
      hash = __hash
    )
  }
  def withTotal(__v: _root_.scala.Int): PartSetHeader = copy(total = __v)
  def withHash(__v: _root_.com.google.protobuf.ByteString): PartSetHeader = copy(hash = __v)

  def getFieldByNumber(__fieldNumber: _root_.scala.Int): _root_.scala.Any = {
    (__fieldNumber: @ _root_.scala.unchecked) match {
      case 1 => {
        val __t = total
        if (__t != 0) __t else null
      }
      case 2 => {
        val __t = hash
        if (__t != _root_.com.google.protobuf.ByteString.EMPTY) __t else null
      }
    }
  }

  def getField(__field: _root_.scalapb.descriptors.FieldDescriptor): _root_.scalapb.descriptors.PValue = {
    _root_.scala.Predef.require(__field.containingMessage eq companion.scalaDescriptor)
    (__field.number: @ _root_.scala.unchecked) match {
      case 1 => _root_.scalapb.descriptors.PInt(total)
      case 2 => _root_.scalapb.descriptors.PByteString(hash)
    }
  }
  def toProtoString: _root_.scala.Predef.String = _root_.scalapb.TextFormat.printToUnicodeString(this)
  def companion = proto3.tendermint.PartSetHeader
}

object PartSetHeader extends scalapb.GeneratedMessageCompanion[proto3.tendermint.PartSetHeader] {
  implicit def messageCompanion: scalapb.GeneratedMessageCompanion[proto3.tendermint.PartSetHeader] = this

  def fromFieldsMap(
    __fieldsMap: scala.collection.immutable.Map[_root_.com.google.protobuf.Descriptors.FieldDescriptor,
                                                _root_.scala.Any]
  ): proto3.tendermint.PartSetHeader = {
    _root_.scala.Predef.require(__fieldsMap.keys.forall(_.getContainingType() == javaDescriptor),
                                "FieldDescriptor does not match message type.")
    val __fields = javaDescriptor.getFields
    proto3.tendermint.PartSetHeader(
      __fieldsMap.getOrElse(__fields.get(0), 0).asInstanceOf[_root_.scala.Int],
      __fieldsMap
        .getOrElse(__fields.get(1), _root_.com.google.protobuf.ByteString.EMPTY)
        .asInstanceOf[_root_.com.google.protobuf.ByteString]
    )
  }
  implicit def messageReads: _root_.scalapb.descriptors.Reads[proto3.tendermint.PartSetHeader] =
    _root_.scalapb.descriptors.Reads {
      case _root_.scalapb.descriptors.PMessage(__fieldsMap) =>
        _root_.scala.Predef.require(__fieldsMap.keys.forall(_.containingMessage == scalaDescriptor),
                                    "FieldDescriptor does not match message type.")
        proto3.tendermint.PartSetHeader(
          __fieldsMap.get(scalaDescriptor.findFieldByNumber(1).get).map(_.as[_root_.scala.Int]).getOrElse(0),
          __fieldsMap
            .get(scalaDescriptor.findFieldByNumber(2).get)
            .map(_.as[_root_.com.google.protobuf.ByteString])
            .getOrElse(_root_.com.google.protobuf.ByteString.EMPTY)
        )
      case _ => throw new RuntimeException("Expected PMessage")
    }

  def javaDescriptor: _root_.com.google.protobuf.Descriptors.Descriptor =
    TendermintProto.javaDescriptor.getMessageTypes.get(0)
  def scalaDescriptor: _root_.scalapb.descriptors.Descriptor = TendermintProto.scalaDescriptor.messages(0)

  def messageCompanionForFieldNumber(__number: _root_.scala.Int): _root_.scalapb.GeneratedMessageCompanion[_] =
    throw new MatchError(__number)
  lazy val nestedMessagesCompanions: Seq[_root_.scalapb.GeneratedMessageCompanion[_]] = Seq.empty

  def enumCompanionForFieldNumber(__fieldNumber: _root_.scala.Int): _root_.scalapb.GeneratedEnumCompanion[_] =
    throw new MatchError(__fieldNumber)
  lazy val defaultInstance = proto3.tendermint.PartSetHeader(
    )
  implicit class PartSetHeaderLens[UpperPB](_l: _root_.scalapb.lenses.Lens[UpperPB, proto3.tendermint.PartSetHeader])
      extends _root_.scalapb.lenses.ObjectLens[UpperPB, proto3.tendermint.PartSetHeader](_l) {
    def total: _root_.scalapb.lenses.Lens[UpperPB, _root_.scala.Int] = field(_.total)((c_, f_) => c_.copy(total = f_))

    def hash: _root_.scalapb.lenses.Lens[UpperPB, _root_.com.google.protobuf.ByteString] =
      field(_.hash)((c_, f_) => c_.copy(hash = f_))
  }
  final val TOTAL_FIELD_NUMBER = 1
  final val HASH_FIELD_NUMBER = 2
}
