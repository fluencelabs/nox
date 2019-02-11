

# Variables

<a id="zero_64"></a>

## `<Const>` ZERO_64

**● ZERO_64**: *"0000000000000000000000000000000000000000000000000000000000000000"* = "0000000000000000000000000000000000000000000000000000000000000000"

*Defined in [util/pad.ts:18](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/util/pad.ts#L18)*

___

# Functions

<a id="padaddress"></a>

## `<Const>` padAddress

▸ **padAddress**(input: *[AddressValue](_types_.md#addressvalue)*): `string`

*Defined in [util/pad.ts:26](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/util/pad.ts#L26)*

Pad an address with zeros on the left.

**Parameters:**

| Name | Type | Description |
| ------ | ------ | ------ |
| input | [AddressValue](_types_.md#addressvalue) |  The input address to pad. |

**Returns:** `string`

___
<a id="padbool"></a>

## `<Const>` padBool

▸ **padBool**(input: *[BoolValue](_types_.md#boolvalue)*): `string`

*Defined in [util/pad.ts:37](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/util/pad.ts#L37)*

Pad a boolean with zeros on the left.

**Parameters:**

| Name | Type | Description |
| ------ | ------ | ------ |
| input | [BoolValue](_types_.md#boolvalue) |  The input address to pad. |

**Returns:** `string`

___
<a id="padbytes"></a>

## `<Const>` padBytes

▸ **padBytes**(input: *[BytesValue](_types_.md#bytesvalue)*): `string`

*Defined in [util/pad.ts:91](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/util/pad.ts#L91)*

Pad bytes with zeros on the left.

**Parameters:**

| Name | Type | Description |
| ------ | ------ | ------ |
| input | [BytesValue](_types_.md#bytesvalue) |  The input bytes to pad. |

**Returns:** `string`

___
<a id="padfixedbytes"></a>

## `<Const>` padFixedBytes

▸ **padFixedBytes**(input: *[BytesValue](_types_.md#bytesvalue)*): `string`

*Defined in [util/pad.ts:102](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/util/pad.ts#L102)*

Pad fixed bytes.

**Parameters:**

| Name | Type | Description |
| ------ | ------ | ------ |
| input | [BytesValue](_types_.md#bytesvalue) |  Input bytes to pad. |

**Returns:** `string`

___
<a id="padstring"></a>

## `<Const>` padString

▸ **padString**(input: *`string`*): `string`

*Defined in [util/pad.ts:117](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/util/pad.ts#L117)*

Pad string.

**Parameters:**

| Name | Type | Description |
| ------ | ------ | ------ |
| input | `string` |  String to pad. |

**Returns:** `string`

___
<a id="padu32"></a>

## `<Const>` padU32

▸ **padU32**(input: *[IntValue](_types_.md#intvalue) | [UintValue](_types_.md#uintvalue)*): `string`

*Defined in [util/pad.ts:46](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/util/pad.ts#L46)*

Pad a u32 with zeros on the left.

**Parameters:**

| Name | Type | Description |
| ------ | ------ | ------ |
| input | [IntValue](_types_.md#intvalue) | [UintValue](_types_.md#uintvalue) |  The input address to pad. |

**Returns:** `string`

___
<a id="stringtobytes"></a>

## `<Const>` stringToBytes

▸ **stringToBytes**(input: *[BytesValue](_types_.md#bytesvalue)*): `number`[]

*Defined in [util/pad.ts:70](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/util/pad.ts#L70)*

Convert an input string to bytes.

**Parameters:**

| Name | Type | Description |
| ------ | ------ | ------ |
| input | [BytesValue](_types_.md#bytesvalue) |  The input string to convert. |

**Returns:** `number`[]

___

