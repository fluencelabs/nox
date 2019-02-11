

# Hierarchy

**Event**

# Constructors

<a id="constructor"></a>

##  constructor

⊕ **new Event**(abi: *[AbiItem](../interfaces/_types_.abiitem.md)*): [Event](_spec_event_event_.event.md)

*Defined in [spec/event/event.ts:20](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/event/event.ts#L20)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| abi | [AbiItem](../interfaces/_types_.abiitem.md) |

**Returns:** [Event](_spec_event_event_.event.md)

___

# Accessors

<a id="anonymous"></a>

##  anonymous

getanonymous(): `boolean`

*Defined in [spec/event/event.ts:36](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/event/event.ts#L36)*

**Returns:** `boolean`

___
<a id="id"></a>

##  id

getid(): `string`

*Defined in [spec/event/event.ts:40](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/event/event.ts#L40)*

**Returns:** `string`

___
<a id="inputs"></a>

##  inputs

getinputs(): [EventParam](_spec_event_eventparam_.eventparam.md)[]

*Defined in [spec/event/event.ts:44](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/event/event.ts#L44)*

**Returns:** [EventParam](_spec_event_eventparam_.eventparam.md)[]

___
<a id="name"></a>

##  name

getname(): `undefined` | `string`

*Defined in [spec/event/event.ts:48](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/event/event.ts#L48)*

**Returns:** `undefined` | `string`

___
<a id="signature"></a>

##  signature

getsignature(): `string`

*Defined in [spec/event/event.ts:52](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/event/event.ts#L52)*

**Returns:** `string`

___

# Methods

<a id="decodelog"></a>

##  decodeLog

▸ **decodeLog**(topics: *`string`[]*, data: *`string`*): [DecodedLog](_spec_event_decodedlog_.decodedlog.md)

*Defined in [spec/event/event.ts:68](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/event/event.ts#L68)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| topics | `string`[] |
| data | `string` |

**Returns:** [DecodedLog](_spec_event_decodedlog_.decodedlog.md)

___
<a id="indexedparams"></a>

##  indexedParams

▸ **indexedParams**(indexed: *`boolean`*): [EventParam](_spec_event_eventparam_.eventparam.md)[]

*Defined in [spec/event/event.ts:64](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/event/event.ts#L64)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| indexed | `boolean` |

**Returns:** [EventParam](_spec_event_eventparam_.eventparam.md)[]

___
<a id="inputparamnames"></a>

##  inputParamNames

▸ **inputParamNames**(): (`undefined` | `string`)[]

*Defined in [spec/event/event.ts:60](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/event/event.ts#L60)*

**Returns:** (`undefined` | `string`)[]

___
<a id="inputparamtypes"></a>

##  inputParamTypes

▸ **inputParamTypes**(): [ParamType](_spec_paramtype_paramtype_.paramtype.md)[]

*Defined in [spec/event/event.ts:56](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/event/event.ts#L56)*

**Returns:** [ParamType](_spec_paramtype_paramtype_.paramtype.md)[]

___

