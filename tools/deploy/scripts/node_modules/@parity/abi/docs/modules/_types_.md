

# Index

### Interfaces

* [AbiInput](../interfaces/_types_.abiinput.md)
* [AbiItem](../interfaces/_types_.abiitem.md)

### Type aliases

* [AbiItemType](_types_.md#abiitemtype)
* [AbiObject](_types_.md#abiobject)
* [AddressValue](_types_.md#addressvalue)
* [ArrayValue](_types_.md#arrayvalue)
* [BoolValue](_types_.md#boolvalue)
* [BytesValue](_types_.md#bytesvalue)
* [FixedArrayValue](_types_.md#fixedarrayvalue)
* [FixedBytesValue](_types_.md#fixedbytesvalue)
* [IntValue](_types_.md#intvalue)
* [MediateType](_types_.md#mediatetype)
* [Slices](_types_.md#slices)
* [StringValue](_types_.md#stringvalue)
* [TokenTypeEnum](_types_.md#tokentypeenum)
* [TokenValue](_types_.md#tokenvalue)
* [UintValue](_types_.md#uintvalue)

---

# Type aliases

<a id="abiitemtype"></a>

##  AbiItemType

**Ƭ AbiItemType**: *"function" | "event" | "constructor" | "fallback"*

*Defined in [types.ts:16](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/types.ts#L16)*

___
<a id="abiobject"></a>

##  AbiObject

**Ƭ AbiObject**: *[AbiItem](../interfaces/_types_.abiitem.md)[]*

*Defined in [types.ts:110](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/types.ts#L110)*

___
<a id="addressvalue"></a>

##  AddressValue

**Ƭ AddressValue**: *`string`*

*Defined in [types.ts:79](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/types.ts#L79)*

___
<a id="arrayvalue"></a>

##  ArrayValue

**Ƭ ArrayValue**: *[FixedArrayValue](_types_.md#fixedarrayvalue)[]*

*Defined in [types.ts:87](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/types.ts#L87)*

___
<a id="boolvalue"></a>

##  BoolValue

**Ƭ BoolValue**: *`boolean` | `string`*

*Defined in [types.ts:80](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/types.ts#L80)*

___
<a id="bytesvalue"></a>

##  BytesValue

**Ƭ BytesValue**: *`string` | `number`[]*

*Defined in [types.ts:81](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/types.ts#L81)*

___
<a id="fixedarrayvalue"></a>

##  FixedArrayValue

**Ƭ FixedArrayValue**: *(`string` | `number` | `false` | `true` | `BigNumber` | [Token](../classes/_token_token_.token.md))[]*

*Defined in [types.ts:86](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/types.ts#L86)*

___
<a id="fixedbytesvalue"></a>

##  FixedBytesValue

**Ƭ FixedBytesValue**: *[BytesValue](_types_.md#bytesvalue)*

*Defined in [types.ts:85](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/types.ts#L85)*

___
<a id="intvalue"></a>

##  IntValue

**Ƭ IntValue**: *`number` | `string` | `BigNumber`*

*Defined in [types.ts:83](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/types.ts#L83)*

___
<a id="mediatetype"></a>

##  MediateType

**Ƭ MediateType**: *"raw" | "prefixed" | "fixedArray" | "array"*

*Defined in [types.ts:18](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/types.ts#L18)*

___
<a id="slices"></a>

##  Slices

**Ƭ Slices**: *`string`[] | `null` | `undefined`*

*Defined in [types.ts:20](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/types.ts#L20)*

___
<a id="stringvalue"></a>

##  StringValue

**Ƭ StringValue**: *`string`*

*Defined in [types.ts:82](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/types.ts#L82)*

___
<a id="tokentypeenum"></a>

##  TokenTypeEnum

**Ƭ TokenTypeEnum**: *"address" | "bool" | "bytes" | "bytes1" | "bytes2" | "bytes3" | "bytes4" | "bytes5" | "bytes6" | "bytes7" | "bytes8" | "bytes9" | "bytes10" | "bytes11" | "bytes12" | "bytes13" | "bytes14" | "bytes15" | "bytes16" | "bytes17" | "bytes18" | "bytes19" | "bytes20" | "bytes21" | "bytes22" | "bytes23" | "bytes24" | "bytes25" | "bytes26" | "bytes27" | "bytes28" | "bytes29" | "bytes30" | "bytes31" | "bytes32" | "string" | "int" | "int8" | "int16" | "int32" | "int64" | "int128" | "int256" | "uint" | "uint8" | "uint16" | "uint32" | "uint64" | "uint128" | "uint256" | "fixedBytes" | "fixedArray" | "array"*

*Defined in [types.ts:23](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/types.ts#L23)*

___
<a id="tokenvalue"></a>

##  TokenValue

**Ƭ TokenValue**: *[AddressValue](_types_.md#addressvalue) | `Boolean` | [BytesValue](_types_.md#bytesvalue) | [StringValue](_types_.md#stringvalue) | [IntValue](_types_.md#intvalue) | [UintValue](_types_.md#uintvalue) | [FixedBytesValue](_types_.md#fixedbytesvalue) | [FixedArrayValue](_types_.md#fixedarrayvalue) | [ArrayValue](_types_.md#arrayvalue)*

*Defined in [types.ts:89](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/types.ts#L89)*

___
<a id="uintvalue"></a>

##  UintValue

**Ƭ UintValue**: *[IntValue](_types_.md#intvalue)*

*Defined in [types.ts:84](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/types.ts#L84)*

___

