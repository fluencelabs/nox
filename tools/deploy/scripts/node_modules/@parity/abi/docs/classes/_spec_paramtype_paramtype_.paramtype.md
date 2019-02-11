

# Hierarchy

**ParamType**

# Constructors

<a id="constructor"></a>

##  constructor

⊕ **new ParamType**(type: *[TokenTypeEnum](../modules/_types_.md#tokentypeenum)*, subtype?: *[ParamType](_spec_paramtype_paramtype_.paramtype.md) | `undefined`*, length?: *`number`*, indexed?: *`boolean`*): [ParamType](_spec_paramtype_paramtype_.paramtype.md)

*Defined in [spec/paramType/paramType.ts:13](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/paramType/paramType.ts#L13)*

**Parameters:**

| Name | Type | Default value |
| ------ | ------ | ------ |
| type | [TokenTypeEnum](../modules/_types_.md#tokentypeenum) | - |
| `Default value` subtype | [ParamType](_spec_paramtype_paramtype_.paramtype.md) | `undefined` |  undefined |
| `Default value` length | `number` | 0 |
| `Default value` indexed | `boolean` | false |

**Returns:** [ParamType](_spec_paramtype_paramtype_.paramtype.md)

___

# Accessors

<a id="indexed"></a>

##  indexed

getindexed(): `undefined` | `false` | `true`

*Defined in [spec/paramType/paramType.ts:49](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/paramType/paramType.ts#L49)*

**Returns:** `undefined` | `false` | `true`

___
<a id="length"></a>

##  length

getlength(): `number`

*Defined in [spec/paramType/paramType.ts:45](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/paramType/paramType.ts#L45)*

**Returns:** `number`

___
<a id="subtype"></a>

##  subtype

getsubtype(): `undefined` | [ParamType](_spec_paramtype_paramtype_.paramtype.md)

*Defined in [spec/paramType/paramType.ts:41](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/paramType/paramType.ts#L41)*

**Returns:** `undefined` | [ParamType](_spec_paramtype_paramtype_.paramtype.md)

___
<a id="type"></a>

##  type

gettype(): "string" | "address" | "bool" | "bytes" | "bytes1" | "bytes2" | "bytes3" | "bytes4" | "bytes5" | "bytes6" | "bytes7" | "bytes8" | "bytes9" | "bytes10" | "bytes11" | "bytes12" | "bytes13" | "bytes14" | "bytes15" | "bytes16" | "bytes17" | "bytes18" | "bytes19" | "bytes20" | "bytes21" | "bytes22" | "bytes23" | "bytes24" | "bytes25" | "bytes26" | "bytes27" | "bytes28" | "bytes29" | "bytes30" | "bytes31" | "bytes32" | "int" | "int8" | "int16" | "int32" | "int64" | "int128" | "int256" | "uint" | "uint8" | "uint16" | "uint32" | "uint64" | "uint128" | "uint256" | "fixedBytes" | "fixedArray" | "array"

*Defined in [spec/paramType/paramType.ts:37](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/paramType/paramType.ts#L37)*

**Returns:** "string" | "address" | "bool" | "bytes" | "bytes1" | "bytes2" | "bytes3" | "bytes4" | "bytes5" | "bytes6" | "bytes7" | "bytes8" | "bytes9" | "bytes10" | "bytes11" | "bytes12" | "bytes13" | "bytes14" | "bytes15" | "bytes16" | "bytes17" | "bytes18" | "bytes19" | "bytes20" | "bytes21" | "bytes22" | "bytes23" | "bytes24" | "bytes25" | "bytes26" | "bytes27" | "bytes28" | "bytes29" | "bytes30" | "bytes31" | "bytes32" | "int" | "int8" | "int16" | "int32" | "int64" | "int128" | "int256" | "uint" | "uint8" | "uint16" | "uint32" | "uint64" | "uint128" | "uint256" | "fixedBytes" | "fixedArray" | "array"

___

# Methods

<a id="validatetype"></a>

## `<Static>` validateType

▸ **validateType**(type: *[TokenTypeEnum](../modules/_types_.md#tokentypeenum)*): `boolean`

*Defined in [spec/paramType/paramType.ts:29](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/paramType/paramType.ts#L29)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| type | [TokenTypeEnum](../modules/_types_.md#tokentypeenum) |

**Returns:** `boolean`

___

