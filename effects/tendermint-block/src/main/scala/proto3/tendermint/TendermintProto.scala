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

object TendermintProto extends _root_.scalapb.GeneratedFileObject {
  lazy val dependencies: Seq[_root_.scalapb.GeneratedFileObject] = Seq(
    com.google.protobuf.timestamp.TimestampProto
  )
  lazy val messagesCompanions: Seq[_root_.scalapb.GeneratedMessageCompanion[_]] = Seq(
    proto3.tendermint.PartSetHeader,
    proto3.tendermint.BlockID,
    proto3.tendermint.Vote,
    proto3.tendermint.Header,
    proto3.tendermint.Version
  )
  private lazy val ProtoBytes: Array[Byte] =
    scalapb.Encoding.fromBase64(
      scala.collection
        .Seq(
          """ChB0ZW5kZXJtaW50LnByb3RvEgZwcm90bzMaH2dvb2dsZS9wcm90b2J1Zi90aW1lc3RhbXAucHJvdG8iOQoNUGFydFNldEhlY
  WRlchIUCgV0b3RhbBgBIAEoBVIFdG90YWwSEgoEaGFzaBgCIAEoDFIEaGFzaCJKCgdCbG9ja0lEEhIKBGhhc2gYASABKAxSBGhhc
  2gSKwoFcGFydHMYAiABKAsyFS5wcm90bzMuUGFydFNldEhlYWRlclIFcGFydHMipQIKBFZvdGUSEgoEdHlwZRgBIAEoBVIEdHlwZ
  RIWCgZoZWlnaHQYAiABKANSBmhlaWdodBIUCgVyb3VuZBgDIAEoBVIFcm91bmQSKwoIYmxvY2tfaWQYBCABKAsyDy5wcm90bzMuQ
  mxvY2tJRFIIYmxvY2tfaWQSOAoJdGltZXN0YW1wGAUgASgLMhouZ29vZ2xlLnByb3RvYnVmLlRpbWVzdGFtcFIJdGltZXN0YW1wE
  iwKEXZhbGlkYXRvcl9hZGRyZXNzGAYgASgMUhF2YWxpZGF0b3JfYWRkcmVzcxIoCg92YWxpZGF0b3JfaW5kZXgYByABKAVSD3Zhb
  GlkYXRvcl9pbmRleBIcCglzaWduYXR1cmUYCCABKAxSCXNpZ25hdHVyZSLhBAoGSGVhZGVyEikKB3ZlcnNpb24YASABKAsyDy5wc
  m90bzMuVmVyc2lvblIHdmVyc2lvbhIZCghjaGFpbl9pZBgCIAEoCVIHY2hhaW5JZBIWCgZoZWlnaHQYAyABKANSBmhlaWdodBIuC
  gR0aW1lGAQgASgLMhouZ29vZ2xlLnByb3RvYnVmLlRpbWVzdGFtcFIEdGltZRIXCgdudW1fdHhzGAUgASgDUgZudW1UeHMSGwoJd
  G90YWxfdHhzGAYgASgDUgh0b3RhbFR4cxIzCg1sYXN0X2Jsb2NrX2lkGAcgASgLMg8ucHJvdG8zLkJsb2NrSURSC2xhc3RCbG9ja
  0lkEigKEGxhc3RfY29tbWl0X2hhc2gYCCABKAxSDmxhc3RDb21taXRIYXNoEhsKCWRhdGFfaGFzaBgJIAEoDFIIZGF0YUhhc2gSJ
  woPdmFsaWRhdG9yc19oYXNoGAogASgMUg52YWxpZGF0b3JzSGFzaBIwChRuZXh0X3ZhbGlkYXRvcnNfaGFzaBgLIAEoDFISbmV4d
  FZhbGlkYXRvcnNIYXNoEiUKDmNvbnNlbnN1c19oYXNoGAwgASgMUg1jb25zZW5zdXNIYXNoEhkKCGFwcF9oYXNoGA0gASgMUgdhc
  HBIYXNoEioKEWxhc3RfcmVzdWx0c19oYXNoGA4gASgMUg9sYXN0UmVzdWx0c0hhc2gSIwoNZXZpZGVuY2VfaGFzaBgPIAEoDFIMZ
  XZpZGVuY2VIYXNoEikKEHByb3Bvc2VyX2FkZHJlc3MYECABKAxSD3Byb3Bvc2VyQWRkcmVzcyIxCgdWZXJzaW9uEhQKBUJsb2NrG
  AEgASgEUgVCbG9jaxIQCgNBcHAYAiABKARSA0FwcGIGcHJvdG8z"""
        )
        .mkString
    )
  lazy val scalaDescriptor: _root_.scalapb.descriptors.FileDescriptor = {
    val scalaProto = com.google.protobuf.descriptor.FileDescriptorProto.parseFrom(ProtoBytes)
    _root_.scalapb.descriptors.FileDescriptor.buildFrom(scalaProto, dependencies.map(_.scalaDescriptor))
  }
  lazy val javaDescriptor: com.google.protobuf.Descriptors.FileDescriptor = {
    val javaProto = com.google.protobuf.DescriptorProtos.FileDescriptorProto.parseFrom(ProtoBytes)
    com.google.protobuf.Descriptors.FileDescriptor.buildFrom(
      javaProto,
      Array(
        com.google.protobuf.timestamp.TimestampProto.javaDescriptor
      )
    )
  }

  @deprecated("Use javaDescriptor instead. In a future version this will refer to scalaDescriptor.", "ScalaPB 0.5.47")
  def descriptor: com.google.protobuf.Descriptors.FileDescriptor = javaDescriptor
}
