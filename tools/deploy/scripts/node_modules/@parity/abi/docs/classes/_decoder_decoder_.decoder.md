

# Hierarchy

**Decoder**

# Methods

<a id="decode"></a>

## `<Static>` decode

▸ **decode**(params: *[ParamType](_spec_paramtype_paramtype_.paramtype.md)[] | `undefined`*, data?: *`undefined` | `string`*): [Token](_token_token_.token.md)[]

*Defined in [decoder/decoder.ts:20](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/decoder/decoder.ts#L20)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| params | [ParamType](_spec_paramtype_paramtype_.paramtype.md)[] | `undefined` |
| `Optional` data | `undefined` | `string` |

**Returns:** [Token](_token_token_.token.md)[]

___
<a id="decodeparam"></a>

## `<Static>` decodeParam

▸ **decodeParam**(param: *[ParamType](_spec_paramtype_paramtype_.paramtype.md)*, slices: *[Slices](../modules/_types_.md#slices)*, offset?: *`number`*): [DecodeResult](_decoder_decoderesult_.decoderesult.md)

*Defined in [decoder/decoder.ts:59](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/decoder/decoder.ts#L59)*

**Parameters:**

| Name | Type | Default value |
| ------ | ------ | ------ |
| param | [ParamType](_spec_paramtype_paramtype_.paramtype.md) | - |
| slices | [Slices](../modules/_types_.md#slices) | - |
| `Default value` offset | `number` | 0 |

**Returns:** [DecodeResult](_decoder_decoderesult_.decoderesult.md)

___
<a id="peek"></a>

## `<Static>` peek

▸ **peek**(slices: *[Slices](../modules/_types_.md#slices)*, position: *`number`*): `string`

*Defined in [decoder/decoder.ts:36](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/decoder/decoder.ts#L36)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| slices | [Slices](../modules/_types_.md#slices) |
| position | `number` |

**Returns:** `string`

___
<a id="takebytes"></a>

## `<Static>` takeBytes

▸ **takeBytes**(slices: *[Slices](../modules/_types_.md#slices)*, position: *`number`*, length: *`number`*): [BytesTaken](_decoder_bytestaken_.bytestaken.md)

*Defined in [decoder/decoder.ts:44](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/decoder/decoder.ts#L44)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| slices | [Slices](../modules/_types_.md#slices) |
| position | `number` |
| length | `number` |

**Returns:** [BytesTaken](_decoder_bytestaken_.bytestaken.md)

___

