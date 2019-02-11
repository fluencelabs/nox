

# Functions

<a id="fromparamtype"></a>

## `<Const>` fromParamType

▸ **fromParamType**(paramType: *[ParamType](../classes/_spec_paramtype_paramtype_.paramtype.md)*): `string`

*Defined in [spec/paramType/format.ts:72](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/paramType/format.ts#L72)*

Convert a ParamType to its string representation.

**Parameters:**

| Name | Type | Description |
| ------ | ------ | ------ |
| paramType | [ParamType](../classes/_spec_paramtype_paramtype_.paramtype.md) |  ParamType instance to convert |

**Returns:** `string`

___
<a id="toparamtype"></a>

## `<Const>` toParamType

▸ **toParamType**(type: *`string`*, indexed?: *`undefined` | `false` | `true`*): [ParamType](../classes/_spec_paramtype_paramtype_.paramtype.md)

*Defined in [spec/paramType/format.ts:15](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/paramType/format.ts#L15)*

Convert a string to a ParamType.

**Parameters:**

| Name | Type | Description |
| ------ | ------ | ------ |
| type | `string` |  Type to convert. |
| `Optional` indexed | `undefined` | `false` | `true` |  Whether the ParamType is indexed or not. |

**Returns:** [ParamType](../classes/_spec_paramtype_paramtype_.paramtype.md)

___

