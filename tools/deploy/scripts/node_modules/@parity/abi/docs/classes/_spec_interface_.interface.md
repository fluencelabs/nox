

# Hierarchy

**Interface**

↳  [Abi](_abi_.abi.md)

# Constructors

<a id="constructor"></a>

##  constructor

⊕ **new Interface**(abi: *[AbiObject](../modules/_types_.md#abiobject)*): [Interface](_spec_interface_.interface.md)

*Defined in [spec/interface.ts:14](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/interface.ts#L14)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| abi | [AbiObject](../modules/_types_.md#abiobject) |

**Returns:** [Interface](_spec_interface_.interface.md)

___

# Accessors

<a id="constructors"></a>

##  constructors

getconstructors(): ([Constructor](_spec_constructor_.constructor.md) | [Event](_spec_event_event_.event.md) | [Func](_spec_function_.func.md))[]

*Defined in [spec/interface.ts:62](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/interface.ts#L62)*

**Returns:** ([Constructor](_spec_constructor_.constructor.md) | [Event](_spec_event_event_.event.md) | [Func](_spec_function_.func.md))[]

___
<a id="events"></a>

##  events

getevents(): ([Constructor](_spec_constructor_.constructor.md) | [Event](_spec_event_event_.event.md) | [Func](_spec_function_.func.md))[]

*Defined in [spec/interface.ts:66](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/interface.ts#L66)*

**Returns:** ([Constructor](_spec_constructor_.constructor.md) | [Event](_spec_event_event_.event.md) | [Func](_spec_function_.func.md))[]

___
<a id="functions"></a>

##  functions

getfunctions(): ([Constructor](_spec_constructor_.constructor.md) | [Event](_spec_event_event_.event.md) | [Func](_spec_function_.func.md))[]

*Defined in [spec/interface.ts:70](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/interface.ts#L70)*

**Returns:** ([Constructor](_spec_constructor_.constructor.md) | [Event](_spec_event_event_.event.md) | [Func](_spec_function_.func.md))[]

___
<a id="interface"></a>

##  interface

getinterface(): ([Constructor](_spec_constructor_.constructor.md) | [Event](_spec_event_event_.event.md) | [Func](_spec_function_.func.md))[]

*Defined in [spec/interface.ts:58](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/interface.ts#L58)*

**Returns:** ([Constructor](_spec_constructor_.constructor.md) | [Event](_spec_event_event_.event.md) | [Func](_spec_function_.func.md))[]

___

# Methods

<a id="encodetokens"></a>

##  encodeTokens

▸ **encodeTokens**(paramTypes: *[ParamType](_spec_paramtype_paramtype_.paramtype.md)[]*, values: *[TokenValue](../modules/_types_.md#tokenvalue)[]*): [Token](_token_token_.token.md)[]

*Defined in [spec/interface.ts:74](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/interface.ts#L74)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| paramTypes | [ParamType](_spec_paramtype_paramtype_.paramtype.md)[] |
| values | [TokenValue](../modules/_types_.md#tokenvalue)[] |

**Returns:** [Token](_token_token_.token.md)[]

___
<a id="encodetokens-1"></a>

## `<Static>` encodeTokens

▸ **encodeTokens**(paramTypes: *[ParamType](_spec_paramtype_paramtype_.paramtype.md)[]*, values: *[TokenValue](../modules/_types_.md#tokenvalue)[]*): [Token](_token_token_.token.md)[]

*Defined in [spec/interface.ts:20](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/interface.ts#L20)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| paramTypes | [ParamType](_spec_paramtype_paramtype_.paramtype.md)[] |
| values | [TokenValue](../modules/_types_.md#tokenvalue)[] |

**Returns:** [Token](_token_token_.token.md)[]

___
<a id="parseabi"></a>

## `<Static>` parseABI

▸ **parseABI**(abi: *[AbiObject](../modules/_types_.md#abiobject)*): ([Constructor](_spec_constructor_.constructor.md) | [Event](_spec_event_event_.event.md) | [Func](_spec_function_.func.md))[]

*Defined in [spec/interface.ts:39](https://github.com/paritytech/js-libs/blob/4f9b60d/packages/abi/src/spec/interface.ts#L39)*

**Parameters:**

| Name | Type |
| ------ | ------ |
| abi | [AbiObject](../modules/_types_.md#abiobject) |

**Returns:** ([Constructor](_spec_constructor_.constructor.md) | [Event](_spec_event_event_.event.md) | [Func](_spec_function_.func.md))[]

___

