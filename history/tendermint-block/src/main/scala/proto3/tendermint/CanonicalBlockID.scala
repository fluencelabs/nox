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
final case class CanonicalBlockID(
  hash: _root_.com.google.protobuf.ByteString = _root_.com.google.protobuf.ByteString.EMPTY,
  partsHeader: _root_.scala.Option[proto3.tendermint.CanonicalPartSetHeader] = None
) extends scalapb.GeneratedMessage with scalapb.Message[CanonicalBlockID]
    with scalapb.lenses.Updatable[CanonicalBlockID] {

  @transient
  private[this] var __serializedSizeCachedValue: _root_.scala.Int = 0
  private[this] def __computeSerializedValue(): _root_.scala.Int = {
    var __size = 0

    {
      val __value = hash
      if (__value != _root_.com.google.protobuf.ByteString.EMPTY) {
        __size += _root_.com.google.protobuf.CodedOutputStream.computeBytesSize(1, __value)
      }
    };
    if (partsHeader.isDefined) {
      val __value = partsHeader.get
      __size += 1 + _root_.com.google.protobuf.CodedOutputStream
        .computeUInt32SizeNoTag(__value.serializedSize) + __value.serializedSize
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
      val __v = hash
      if (__v != _root_.com.google.protobuf.ByteString.EMPTY) {
        _output__.writeBytes(1, __v)
      }
    };
    partsHeader.foreach { __v =>
      val __m = __v
      _output__.writeTag(2, 2)
      _output__.writeUInt32NoTag(__m.serializedSize)
      __m.writeTo(_output__)
    };
  }

  def mergeFrom(`_input__`: _root_.com.google.protobuf.CodedInputStream): proto3.tendermint.CanonicalBlockID = {
    var __hash = this.hash
    var __partsHeader = this.partsHeader
    var _done__ = false
    while (!_done__) {
      val _tag__ = _input__.readTag()
      _tag__ match {
        case 0 => _done__ = true
        case 10 =>
          __hash = _input__.readBytes()
        case 18 =>
          __partsHeader = Option(
            _root_.scalapb.LiteParser
              .readMessage(_input__, __partsHeader.getOrElse(proto3.tendermint.CanonicalPartSetHeader.defaultInstance))
          )
        case tag => _input__.skipField(tag)
      }
    }
    proto3.tendermint.CanonicalBlockID(
      hash = __hash,
      partsHeader = __partsHeader
    )
  }
  def withHash(__v: _root_.com.google.protobuf.ByteString): CanonicalBlockID = copy(hash = __v)

  def getPartsHeader: proto3.tendermint.CanonicalPartSetHeader =
    partsHeader.getOrElse(proto3.tendermint.CanonicalPartSetHeader.defaultInstance)
  def clearPartsHeader: CanonicalBlockID = copy(partsHeader = None)
  def withPartsHeader(__v: proto3.tendermint.CanonicalPartSetHeader): CanonicalBlockID = copy(partsHeader = Option(__v))

  def getFieldByNumber(__fieldNumber: _root_.scala.Int): _root_.scala.Any = {
    (__fieldNumber: @ _root_.scala.unchecked) match {
      case 1 => {
        val __t = hash
        if (__t != _root_.com.google.protobuf.ByteString.EMPTY) __t else null
      }
      case 2 => partsHeader.orNull
    }
  }

  def getField(__field: _root_.scalapb.descriptors.FieldDescriptor): _root_.scalapb.descriptors.PValue = {
    _root_.scala.Predef.require(__field.containingMessage eq companion.scalaDescriptor)
    (__field.number: @ _root_.scala.unchecked) match {
      case 1 => _root_.scalapb.descriptors.PByteString(hash)
      case 2 => partsHeader.map(_.toPMessage).getOrElse(_root_.scalapb.descriptors.PEmpty)
    }
  }
  def toProtoString: _root_.scala.Predef.String = _root_.scalapb.TextFormat.printToUnicodeString(this)
  def companion = proto3.tendermint.CanonicalBlockID
}

object CanonicalBlockID extends scalapb.GeneratedMessageCompanion[proto3.tendermint.CanonicalBlockID] {
  implicit def messageCompanion: scalapb.GeneratedMessageCompanion[proto3.tendermint.CanonicalBlockID] = this

  def fromFieldsMap(
    __fieldsMap: scala.collection.immutable.Map[
      _root_.com.google.protobuf.Descriptors.FieldDescriptor,
      _root_.scala.Any
    ]
  ): proto3.tendermint.CanonicalBlockID = {
    _root_.scala.Predef.require(
      __fieldsMap.keys.forall(_.getContainingType() == javaDescriptor),
      "FieldDescriptor does not match message type."
    )
    val __fields = javaDescriptor.getFields
    proto3.tendermint.CanonicalBlockID(
      __fieldsMap
        .getOrElse(__fields.get(0), _root_.com.google.protobuf.ByteString.EMPTY)
        .asInstanceOf[_root_.com.google.protobuf.ByteString],
      __fieldsMap.get(__fields.get(1)).asInstanceOf[_root_.scala.Option[proto3.tendermint.CanonicalPartSetHeader]]
    )
  }
  implicit def messageReads: _root_.scalapb.descriptors.Reads[proto3.tendermint.CanonicalBlockID] =
    _root_.scalapb.descriptors.Reads {
      case _root_.scalapb.descriptors.PMessage(__fieldsMap) =>
        _root_.scala.Predef.require(
          __fieldsMap.keys.forall(_.containingMessage == scalaDescriptor),
          "FieldDescriptor does not match message type."
        )
        proto3.tendermint.CanonicalBlockID(
          __fieldsMap
            .get(scalaDescriptor.findFieldByNumber(1).get)
            .map(_.as[_root_.com.google.protobuf.ByteString])
            .getOrElse(_root_.com.google.protobuf.ByteString.EMPTY),
          __fieldsMap
            .get(scalaDescriptor.findFieldByNumber(2).get)
            .flatMap(_.as[_root_.scala.Option[proto3.tendermint.CanonicalPartSetHeader]])
        )
      case _ => throw new RuntimeException("Expected PMessage")
    }

  def javaDescriptor: _root_.com.google.protobuf.Descriptors.Descriptor =
    TendermintProto.javaDescriptor.getMessageTypes.get(14)
  def scalaDescriptor: _root_.scalapb.descriptors.Descriptor = TendermintProto.scalaDescriptor.messages(14)

  def messageCompanionForFieldNumber(__number: _root_.scala.Int): _root_.scalapb.GeneratedMessageCompanion[_] = {
    var __out: _root_.scalapb.GeneratedMessageCompanion[_] = null
    (__number: @ _root_.scala.unchecked) match {
      case 2 => __out = proto3.tendermint.CanonicalPartSetHeader
    }
    __out
  }
  lazy val nestedMessagesCompanions: Seq[_root_.scalapb.GeneratedMessageCompanion[_]] = Seq.empty

  def enumCompanionForFieldNumber(__fieldNumber: _root_.scala.Int): _root_.scalapb.GeneratedEnumCompanion[_] =
    throw new MatchError(__fieldNumber)
  lazy val defaultInstance = proto3.tendermint.CanonicalBlockID(
    )
  implicit class CanonicalBlockIDLens[UpperPB](
    _l: _root_.scalapb.lenses.Lens[UpperPB, proto3.tendermint.CanonicalBlockID]
  ) extends _root_.scalapb.lenses.ObjectLens[UpperPB, proto3.tendermint.CanonicalBlockID](_l) {

    def hash: _root_.scalapb.lenses.Lens[UpperPB, _root_.com.google.protobuf.ByteString] =
      field(_.hash)((c_, f_) => c_.copy(hash = f_))

    def partsHeader: _root_.scalapb.lenses.Lens[UpperPB, proto3.tendermint.CanonicalPartSetHeader] =
      field(_.getPartsHeader)((c_, f_) => c_.copy(partsHeader = Option(f_)))

    def optionalPartsHeader
      : _root_.scalapb.lenses.Lens[UpperPB, _root_.scala.Option[proto3.tendermint.CanonicalPartSetHeader]] =
      field(_.partsHeader)((c_, f_) => c_.copy(partsHeader = f_))
  }
  final val HASH_FIELD_NUMBER = 1
  final val PARTS_HEADER_FIELD_NUMBER = 2
}
