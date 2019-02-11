

# Hierarchy

**EventParam**

# Constructors

<a id="constructor"></a>

##  constructor

⊕ **new EventParam**(name: *`string` | `undefined`*, type: *[TokenTypeEnum](../modules/_types_.md#tokentypeenum)*, indexed?: *`boolean`*): [EventParam](_spec_event_eventparam_.eventparam.md)

*Defined in [spec/event/eventParam.ts:27](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/event/eventParam.ts#L27)*

**Parameters:**

| Name | Type | Default value |
| ------ | ------ | ------ |
| name | `string` | `undefined` | - |
| type | [TokenTypeEnum](../modules/_types_.md#tokentypeenum) | - |
| `Default value` indexed | `boolean` | false |

**Returns:** [EventParam](_spec_event_eventparam_.eventparam.md)

___

# Accessors

<a id="indexed"></a>

##  indexed

getindexed(): `boolean`

*Defined in [spec/event/eventParam.ts:43](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/event/eventParam.ts#L43)*

**Returns:** `boolean`

___
<a id="kind"></a>

##  kind

getkind(): [ParamType](_spec_paramtype_paramtype_.paramtype.md)

*Defined in [spec/event/eventParam.ts:39](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/event/eventParam.ts#L39)*

**Returns:** [ParamType](_spec_paramtype_paramtype_.paramtype.md)

___
<a id="name"></a>

##  name

getname(): `undefined` | `string`

*Defined in [spec/event/eventParam.ts:35](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/event/eventParam.ts#L35)*

**Returns:** `undefined` | `string`

___

# Methods

<a id="toeventparams"></a>

## `<Static>` toEventParams

▸ **toEventParams**(params: *([AbiInput](../interfaces/_types_.abiinput.md) | [Param](_spec_param_.param.md))[]*): [EventParam](_spec_event_eventparam_.eventparam.md)[]

*Defined in [spec/event/eventParam.ts:16](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/event/eventParam.ts#L16)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| params | ([AbiInput](../interfaces/_types_.abiinput.md) | [Param](_spec_param_.param.md))[] |

**Returns:** [EventParam](_spec_event_eventparam_.eventparam.md)[]

___

