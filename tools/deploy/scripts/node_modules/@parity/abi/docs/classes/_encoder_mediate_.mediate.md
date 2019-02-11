

# Hierarchy

**Mediate**

# Constructors

<a id="constructor"></a>

##  constructor

⊕ **new Mediate**(type: *[MediateType](../modules/_types_.md#mediatetype)*, value: *[MediateValue](../modules/_encoder_mediate_.md#mediatevalue)*): [Mediate](_encoder_mediate_.mediate.md)

*Defined in [encoder/mediate.ts:15](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/encoder/mediate.ts#L15)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| type | [MediateType](../modules/_types_.md#mediatetype) |
| value | [MediateValue](../modules/_encoder_mediate_.md#mediatevalue) |

**Returns:** [Mediate](_encoder_mediate_.mediate.md)

___

# Accessors

<a id="type"></a>

##  type

gettype(): "fixedArray" | "array" | "raw" | "prefixed"

*Defined in [encoder/mediate.ts:150](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/encoder/mediate.ts#L150)*

**Returns:** "fixedArray" | "array" | "raw" | "prefixed"

___
<a id="value"></a>

##  value

getvalue(): `string` | `number` | `Boolean` | `number`[] | `BigNumber` | (`string` | `number` | `false` | `true` | `BigNumber` | [Token](_token_token_.token.md))[] | (`string` | `number` | `false` | `true` | `BigNumber` | [Token](_token_token_.token.md))[][] | [Mediate](_encoder_mediate_.mediate.md)[]

*Defined in [encoder/mediate.ts:154](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/encoder/mediate.ts#L154)*

**Returns:** `string` | `number` | `Boolean` | `number`[] | `BigNumber` | (`string` | `number` | `false` | `true` | `BigNumber` | [Token](_token_token_.token.md))[] | (`string` | `number` | `false` | `true` | `BigNumber` | [Token](_token_token_.token.md))[][] | [Mediate](_encoder_mediate_.mediate.md)[]

___

# Methods

<a id="closing"></a>

##  closing

▸ **closing**(offset: *`number`*): `string`

*Defined in [encoder/mediate.ts:105](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/encoder/mediate.ts#L105)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| offset | `number` |

**Returns:** `string`

___
<a id="closinglength"></a>

##  closingLength

▸ **closingLength**(): `number`

*Defined in [encoder/mediate.ts:64](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/encoder/mediate.ts#L64)*

**Returns:** `number`

___
<a id="init"></a>

##  init

▸ **init**(suffixOffset: *`number`*): `string`

*Defined in [encoder/mediate.ts:84](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/encoder/mediate.ts#L84)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| suffixOffset | `number` |

**Returns:** `string`

___
<a id="initlength"></a>

##  initLength

▸ **initLength**(): `number`

*Defined in [encoder/mediate.ts:48](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/encoder/mediate.ts#L48)*

**Returns:** `number`

___
<a id="offsetfor"></a>

## `<Static>` offsetFor

▸ **offsetFor**(mediates: *[Mediate](_encoder_mediate_.mediate.md)[]*, position: *`number`*): `number`

*Defined in [encoder/mediate.ts:24](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/encoder/mediate.ts#L24)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| mediates | [Mediate](_encoder_mediate_.mediate.md)[] |
| position | `number` |

**Returns:** `number`

___
<a id="validatetype"></a>

## `<Static>` validateType

▸ **validateType**(type: *[MediateType](../modules/_types_.md#mediatetype)*): `boolean`

*Defined in [encoder/mediate.ts:40](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/encoder/mediate.ts#L40)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| type | [MediateType](../modules/_types_.md#mediatetype) |

**Returns:** `boolean`

___

