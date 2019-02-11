

# Hierarchy

**Func**

# Constructors

<a id="constructor"></a>

##  constructor

⊕ **new Func**(abi: *[AbiItem](../interfaces/_types_.abiitem.md)*): [Func](_spec_function_.func.md)

*Defined in [spec/function.ts:21](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/function.ts#L21)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| abi | [AbiItem](../interfaces/_types_.abiitem.md) |

**Returns:** [Func](_spec_function_.func.md)

___

# Accessors

<a id="abi"></a>

##  abi

getabi(): [AbiItem](../interfaces/_types_.abiitem.md)

*Defined in [spec/function.ts:40](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/function.ts#L40)*

**Returns:** [AbiItem](../interfaces/_types_.abiitem.md)

___
<a id="constant"></a>

##  constant

getconstant(): `boolean`

*Defined in [spec/function.ts:44](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/function.ts#L44)*

**Returns:** `boolean`

___
<a id="id"></a>

##  id

getid(): `string`

*Defined in [spec/function.ts:48](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/function.ts#L48)*

**Returns:** `string`

___
<a id="inputs"></a>

##  inputs

getinputs(): [Param](_spec_param_.param.md)[]

*Defined in [spec/function.ts:52](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/function.ts#L52)*

**Returns:** [Param](_spec_param_.param.md)[]

___
<a id="name"></a>

##  name

getname(): `undefined` | `string`

*Defined in [spec/function.ts:56](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/function.ts#L56)*

**Returns:** `undefined` | `string`

___
<a id="outputs"></a>

##  outputs

getoutputs(): [Param](_spec_param_.param.md)[]

*Defined in [spec/function.ts:60](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/function.ts#L60)*

**Returns:** [Param](_spec_param_.param.md)[]

___
<a id="payable"></a>

##  payable

getpayable(): `boolean`

*Defined in [spec/function.ts:64](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/function.ts#L64)*

**Returns:** `boolean`

___
<a id="signature"></a>

##  signature

getsignature(): `string`

*Defined in [spec/function.ts:68](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/function.ts#L68)*

**Returns:** `string`

___

# Methods

<a id="decodeinput"></a>

##  decodeInput

▸ **decodeInput**(data?: *`undefined` | `string`*): [Token](_token_token_.token.md)[]

*Defined in [spec/function.ts:72](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/function.ts#L72)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| `Optional` data | `undefined` | `string` |

**Returns:** [Token](_token_token_.token.md)[]

___
<a id="decodeoutput"></a>

##  decodeOutput

▸ **decodeOutput**(data?: *`undefined` | `string`*): [Token](_token_token_.token.md)[]

*Defined in [spec/function.ts:76](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/function.ts#L76)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| `Optional` data | `undefined` | `string` |

**Returns:** [Token](_token_token_.token.md)[]

___
<a id="encodecall"></a>

##  encodeCall

▸ **encodeCall**(tokens: *[Token](_token_token_.token.md)[]*): `string`

*Defined in [spec/function.ts:80](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/function.ts#L80)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| tokens | [Token](_token_token_.token.md)[] |

**Returns:** `string`

___
<a id="inputparamtypes"></a>

##  inputParamTypes

▸ **inputParamTypes**(): [ParamType](_spec_paramtype_paramtype_.paramtype.md)[]

*Defined in [spec/function.ts:84](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/function.ts#L84)*

**Returns:** [ParamType](_spec_paramtype_paramtype_.paramtype.md)[]

___
<a id="outputparamtypes"></a>

##  outputParamTypes

▸ **outputParamTypes**(): [ParamType](_spec_paramtype_paramtype_.paramtype.md)[]

*Defined in [spec/function.ts:88](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/function.ts#L88)*

**Returns:** [ParamType](_spec_paramtype_paramtype_.paramtype.md)[]

___

