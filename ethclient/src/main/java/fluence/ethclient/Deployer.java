/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.ethclient;

import java.math.BigInteger;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.concurrent.Callable;
import org.web3j.abi.EventEncoder;
import org.web3j.abi.TypeReference;
import org.web3j.abi.datatypes.Address;
import org.web3j.abi.datatypes.Bool;
import org.web3j.abi.datatypes.DynamicArray;
import org.web3j.abi.datatypes.Event;
import org.web3j.abi.datatypes.Function;
import org.web3j.abi.datatypes.Type;
import org.web3j.abi.datatypes.Utf8String;
import org.web3j.abi.datatypes.generated.Bytes24;
import org.web3j.abi.datatypes.generated.Bytes32;
import org.web3j.abi.datatypes.generated.Uint16;
import org.web3j.abi.datatypes.generated.Uint256;
import org.web3j.abi.datatypes.generated.Uint8;
import org.web3j.crypto.Credentials;
import org.web3j.protocol.Web3j;
import org.web3j.protocol.core.DefaultBlockParameter;
import org.web3j.protocol.core.RemoteCall;
import org.web3j.protocol.core.methods.request.EthFilter;
import org.web3j.protocol.core.methods.response.Log;
import org.web3j.protocol.core.methods.response.TransactionReceipt;
import org.web3j.tuples.generated.Tuple3;
import org.web3j.tuples.generated.Tuple6;
import org.web3j.tx.Contract;
import org.web3j.tx.TransactionManager;
import org.web3j.tx.gas.ContractGasProvider;
import rx.Observable;
import rx.functions.Func1;

/**
 * <p>Auto generated code.
 * <p><strong>Do not modify!</strong>
 * <p>Please use the <a href="https://docs.web3j.io/command_line.html">web3j command line tools</a>,
 * or the org.web3j.codegen.SolidityFunctionWrapperGenerator in the 
 * <a href="https://github.com/web3j/web3j/tree/master/codegen">codegen module</a> to update.
 *
 * <p>Generated with web3j version 3.6.0.
 */
public class Deployer extends Contract {
    private static final String BINARY = "60806040526001600755336000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff160217905550612621806100586000396000f3006080604052600436106100e6576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff1680630988ca8c146100eb57806318b919e914610174578063217fe6c6146102045780632238ba2f146102a557806324953eaa146102f1578063286dd3f5146103575780634e69d5601461039a578063715018a61461041a5780637b9417c8146104315780637ea29f62146104745780638da5cb5b146104d65780639b19251a1461052d578063c7c02e4414610588578063e2683e921461060e578063e2ec6ec314610749578063f2fde38b146107af575b600080fd5b3480156100f757600080fd5b50610172600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803590602001908201803590602001908080601f01602080910402602001604051908101604052809392919081815260200183838082843782019150505050505091929192905050506107f2565b005b34801561018057600080fd5b50610189610873565b6040518080602001828103825283818151815260200191508051906020019080838360005b838110156101c95780820151818401526020810190506101ae565b50505050905090810190601f1680156101f65780820380516001836020036101000a031916815260200191505b509250505060405180910390f35b34801561021057600080fd5b5061028b600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803590602001908201803590602001908080601f01602080910402602001604051908101604052809392919081815260200183838082843782019150505050505091929192905050506108ac565b604051808215151515815260200191505060405180910390f35b3480156102b157600080fd5b506102ef60048036038101908080356000191690602001909291908035600019169060200190929190803560ff169060200190929190505050610933565b005b3480156102fd57600080fd5b5061035560048036038101908080359060200190820180359060200190808060200260200160405190810160405280939291908181526020018383602002808284378201915050505050509192919290505050610a32565b005b34801561036357600080fd5b50610398600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610ace565b005b3480156103a657600080fd5b506103af610b6b565b604051808460ff1660ff16815260200183815260200180602001828103825283818151815260200191508051906020019060200280838360005b838110156104045780820151818401526020810190506103e9565b5050505090500194505050505060405180910390f35b34801561042657600080fd5b5061042f610c2f565b005b34801561043d57600080fd5b50610472600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610d31565b005b34801561048057600080fd5b506104d46004803603810190808035600019169060200190929190803567ffffffffffffffff19169060200190929190803561ffff169060200190929190803561ffff169060200190929190505050610dce565b005b3480156104e257600080fd5b506104eb6110d6565b604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390f35b34801561053957600080fd5b5061056e600480360381019080803573ffffffffffffffffffffffffffffffffffffffff1690602001909291905050506110fb565b604051808215151515815260200191505060405180910390f35b34801561059457600080fd5b506105b76004803603810190808035600019169060200190929190505050611143565b6040518080602001828103825283818151815260200191508051906020019060200280838360005b838110156105fa5780820151818401526020810190506105df565b505050509050019250505060405180910390f35b34801561061a57600080fd5b5061063d60048036038101908080356000191690602001909291905050506112d7565b6040518087600019166000191681526020018660001916600019168152602001858152602001806020018060200180602001848103845287818151815260200191508051906020019060200280838360005b838110156106aa57808201518184015260208101905061068f565b50505050905001848103835286818151815260200191508051906020019060200280838360005b838110156106ec5780820151818401526020810190506106d1565b50505050905001848103825285818151815260200191508051906020019060200280838360005b8381101561072e578082015181840152602081019050610713565b50505050905001995050505050505050505060405180910390f35b34801561075557600080fd5b506107ad60048036038101908080359060200190820180359060200190808060200260200160405190810160405280939291908181526020018383602002808284378201915050505050509192919290505050611655565b005b3480156107bb57600080fd5b506107f0600480360381019080803573ffffffffffffffffffffffffffffffffffffffff1690602001909291905050506116f1565b005b61086f826001836040518082805190602001908083835b60208310151561082e5780518252602082019150602081019050602083039250610809565b6001836020036101000a038019825116818451168082178552505050505050905001915050908152602001604051809103902061175890919063ffffffff16565b5050565b6040805190810160405280600981526020017f77686974656c697374000000000000000000000000000000000000000000000081525081565b600061092b836001846040518082805190602001908083835b6020831015156108ea57805182526020820191506020810190506020830392506108c5565b6001836020036101000a038019825116818451168082178552505050505050905001915050908152602001604051809103902061177190919063ffffffff16565b905092915050565b61093c336110fb565b151561094757600080fd5b600860606040519081016040528085600019168152602001846000191681526020018360ff1681525090806001815401808255809150509060018203906000526020600020906003020160009091929091909150600082015181600001906000191690556020820151816001019060001916905560408201518160020160006101000a81548160ff021916908360ff1602179055505050506109e76117ca565b1515610a2d577fd18fba5b22517a48b063e62f8b6acbfc4dbfba1583e929178d3fc862218544dd8360405180826000191660001916815260200191505060405180910390a15b505050565b60008060009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16141515610a8f57600080fd5b600090505b8151811015610aca57610abd8282815181101515610aae57fe5b90602001906020020151610ace565b8080600101915050610a94565b5050565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16141515610b2957600080fd5b610b68816040805190810160405280600981526020017f77686974656c6973740000000000000000000000000000000000000000000000815250611e2f565b50565b600080606080600080600880549050604051908082528060200260200182016040528015610ba85781602001602082028038833980820191505090505b509250600091505b600880549050821015610c1557600882815481101515610bcc57fe5b906000526020600020906003020160020160009054906101000a900460ff1660ff168383815181101515610bfc57fe5b9060200190602002018181525050816001019150610bb0565b606590508060028054905084955095509550505050909192565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16141515610c8a57600080fd5b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff167ff8df31144d9c2f0f6b59d69b8b98abd5459d07f2742c4df920b25aae33c6482060405160405180910390a260008060006101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff160217905550565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16141515610d8c57600080fd5b610dcb816040805190810160405280600981526020017f77686974656c6973740000000000000000000000000000000000000000000000815250611f63565b50565b610dd7336110fb565b1515610de257600080fd5b60006001026003600086600019166000191681526020019081526020016000206000015460001916141515610e7f576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040180806020018281038252601f8152602001807f54686973206e6f646520697320616c726561647920726567697374657265640081525060200191505060405180910390fd5b8061ffff168261ffff16101515610efe576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260208152602001807f506f72742072616e676520697320656d707479206f7220696e636f727265637481525060200191505060405180910390fd5b60c060405190810160405280856000191681526020018467ffffffffffffffff191681526020018361ffff1681526020018261ffff1681526020018361ffff1681526020016006805490508152506003600086600019166000191681526020019081526020016000206000820151816000019060001916905560208201518160010160006101000a81548177ffffffffffffffffffffffffffffffffffffffffffffffff0219169083680100000000000000009004021790555060408201518160010160186101000a81548161ffff021916908361ffff160217905550606082015181600101601a6101000a81548161ffff021916908361ffff160217905550608082015181600101601c6101000a81548161ffff021916908361ffff16021790555060a08201518160020155905050600284908060018154018082558091505090600182039060005260206000200160009091929091909150906000191690555081810361ffff1660068181805490500191508161107d9190612435565b507fb0cd47a7093fb93a9ce97304d3afb8df43e02e48502e47fd5fbb6c4020d935b58460405180826000191660001916815260200191505060405180910390a15b6110c66117ca565b156110d0576110be565b50505050565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b600061113c826040805190810160405280600981526020017f77686974656c69737400000000000000000000000000000000000000000000008152506108ac565b9050919050565b606061114d612461565b6060600060036000866000191660001916815260200190815260200160002060c060405190810160405290816000820154600019166000191681526020016001820160009054906101000a9004680100000000000000000267ffffffffffffffff191667ffffffffffffffff191681526020016001820160189054906101000a900461ffff1661ffff1661ffff16815260200160018201601a9054906101000a900461ffff1661ffff1661ffff16815260200160018201601c9054906101000a900461ffff1661ffff1661ffff1681526020016002820154815250509250826040015183608001510361ffff166040519080825280602002602001820160405280156112685781602001602082028038833980820191505090505b509150600090505b81518110156112cc576006818460a001510181548110151561128e57fe5b906000526020600020015482828151811015156112a757fe5b9060200190602002019060001916908160001916815250508080600101915050611270565b819350505050919050565b600080600060608060606112e96124b2565b606080606060006112f86124e4565b600560008e600019166000191681526020019081526020016000206080604051908101604052908160008201546000191660001916815260200160018201606060405190810160405290816000820154600019166000191681526020016001820154600019166000191681526020016002820160009054906101000a900460ff1660ff1660ff168152505081526020016004820154815260200160058201548152505095506000600102866000015160001916111515611420576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260188152602001807f7468657265206973206e6f207375636820636c7573746572000000000000000081525060200191505060405180910390fd5b85602001516040015160ff1660405190808252806020026020018201604052801561145a5781602001602082028038833980820191505090505b50945085602001516040015160ff166040519080825280602002602001820160405280156114975781602001602082028038833980820191505090505b50935085602001516040015160ff166040519080825280602002602001820160405280156114d45781602001602082028038833980820191505090505b509250600091505b85602001516040015160ff168210156116205760048287606001510181548110151561150457fe5b90600052602060002090600202016040805190810160405290816000820154600019166000191681526020016001820160009054906101000a900461ffff1661ffff1661ffff168152505090508060000151858381518110151561156457fe5b9060200190602002019060001916908160001916815250506003600082600001516000191660001916815260200190815260200160002060010160009054906101000a9004680100000000000000000284838151811015156115c257fe5b9060200190602002019067ffffffffffffffff1916908167ffffffffffffffff191681525050806020015183838151811015156115fb57fe5b9060200190602002019061ffff16908161ffff168152505081806001019250506114dc565b85602001516000015186602001516020015187604001518787879b509b509b509b509b509b5050505050505091939550919395565b60008060009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff161415156116b257600080fd5b600090505b81518110156116ed576116e082828151811015156116d157fe5b90602001906020020151610d31565b80806001019150506116b7565b5050565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff1614151561174c57600080fd5b61175581612097565b50565b6117628282611771565b151561176d57600080fd5b5050565b60008260000160008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060009054906101000a900460ff16905092915050565b6000806117d5612505565b600080606080606060008060006117ea612461565b6117f26124e4565b60009b505b6008805490508c10156118525760088c81548110151561181357fe5b906000526020600020906003020160020160009054906101000a900460ff1660ff1660028054905010151561184757611852565b8b6001019b506117f7565b6008805490508c1015156118695760009c50611e20565b60088c81548110151561187857fe5b9060005260206000209060030201606060405190810160405290816000820154600019166000191681526020016001820154600019166000191681526020016002820160009054906101000a900460ff1660ff1660ff16815250509a506118de8c612191565b600760008154809291906001019190505560010299504298506080604051908101604052808b6000191681526020018c81526020018a8152602001600480549050815250600560008c6000191660001916815260200190815260200160002060008201518160000190600019169055602082015181600101600082015181600001906000191690556020820151816001019060001916905560408201518160020160006101000a81548160ff021916908360ff160217905550505060408201518160040155606082015181600501559050508a6040015160ff166040519080825280602002602001820160405280156119e65781602001602082028038833980820191505090505b5097508a6040015160ff16604051908082528060200260200182016040528015611a1f5781602001602082028038833980820191505090505b5096508a6040015160ff16604051908082528060200260200182016040528015611a585781602001602082028038833980820191505090505b50955060009450600093505b8a6040015160ff16841015611ce557600285815481101515611a8257fe5b9060005260206000200154925060036000846000191660001916815260200190815260200160002060c060405190810160405290816000820154600019166000191681526020016001820160009054906101000a9004680100000000000000000267ffffffffffffffff191667ffffffffffffffff191681526020016001820160189054906101000a900461ffff1661ffff1661ffff16815260200160018201601a9054906101000a900461ffff1661ffff1661ffff16815260200160018201601c9054906101000a900461ffff1661ffff1661ffff1681526020016002820154815250509150604080519081016040528084600019168152602001836080015161ffff168152509050600481908060018154018082558091505090600182039060005260206000209060020201600090919290919091506000820151816000019060001916905560208201518160010160006101000a81548161ffff021916908361ffff160217905550505050896006836040015161ffff16846080015161ffff168560a001510103815481101515611c1857fe5b906000526020600020018160001916905550828885815181101515611c3957fe5b90602001906020020190600019169081600019168152505081602001518785815181101515611c6457fe5b9060200190602002019067ffffffffffffffff1916908167ffffffffffffffff19168152505080602001518685815181101515611c9d57fe5b9060200190602002019061ffff16908161ffff1681525050611cbe8361224e565b15611cce57846001019450611cd8565b611cd78561230c565b5b8380600101945050611a64565b7f28c3d361196410d2059b40d53bf75ae21adebcec217c5a2564746ed2c3427fd28a8c600001518b8b8b8b6040518087600019166000191681526020018660001916600019168152602001858152602001806020018060200180602001848103845287818151815260200191508051906020019060200280838360005b83811015611d7d578082015181840152602081019050611d62565b50505050905001848103835286818151815260200191508051906020019060200280838360005b83811015611dbf578082015181840152602081019050611da4565b50505050905001848103825285818151815260200191508051906020019060200280838360005b83811015611e01578082015181840152602081019050611de6565b50505050905001995050505050505050505060405180910390a160019c505b50505050505050505050505090565b611eac826001836040518082805190602001908083835b602083101515611e6b5780518252602082019150602081019050602083039250611e46565b6001836020036101000a038019825116818451168082178552505050505050905001915050908152602001604051809103902061237990919063ffffffff16565b8173ffffffffffffffffffffffffffffffffffffffff167fd211483f91fc6eff862467f8de606587a30c8fc9981056f051b897a418df803a826040518080602001828103825283818151815260200191508051906020019080838360005b83811015611f25578082015181840152602081019050611f0a565b50505050905090810190601f168015611f525780820380516001836020036101000a031916815260200191505b509250505060405180910390a25050565b611fe0826001836040518082805190602001908083835b602083101515611f9f5780518252602082019150602081019050602083039250611f7a565b6001836020036101000a03801982511681845116808217855250505050505090500191505090815260200160405180910390206123d790919063ffffffff16565b8173ffffffffffffffffffffffffffffffffffffffff167fbfec83d64eaa953f2708271a023ab9ee82057f8f3578d548c1a4ba0b5b700489826040518080602001828103825283818151815260200191508051906020019080838360005b8381101561205957808201518184015260208101905061203e565b50505050905090810190601f1680156120865780820380516001836020036101000a031916815260200191505b509250505060405180910390a25050565b600073ffffffffffffffffffffffffffffffffffffffff168173ffffffffffffffffffffffffffffffffffffffff16141515156120d357600080fd5b8073ffffffffffffffffffffffffffffffffffffffff166000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff167f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e060405160405180910390a3806000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555050565b600160088054905003811415156122365760086001600880549050038154811015156121b957fe5b90600052602060002090600302016008828154811015156121d657fe5b906000526020600020906003020160008201548160000190600019169055600182015481600101906000191690556002820160009054906101000a900460ff168160020160006101000a81548160ff021916908360ff1602179055509050505b6008805460019003908161224a9190612530565b5050565b6000600360008360001916600019168152602001908152602001600020600101601c81819054906101000a900461ffff168092919060010191906101000a81548161ffff021916908361ffff16021790555050600360008360001916600019168152602001908152602001600020600101601a9054906101000a900461ffff1661ffff16600360008460001916600019168152602001908152602001600020600101601c9054906101000a900461ffff1661ffff1614159050919050565b6001600280549050038114151561236157600260016002805490500381548110151561233457fe5b906000526020600020015460028281548110151561234e57fe5b9060005260206000200181600019169055505b600280546001900390816123759190612435565b5050565b60008260000160008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060006101000a81548160ff0219169083151502179055505050565b60018260000160008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060006101000a81548160ff0219169083151502179055505050565b81548183558181111561245c5781836000526020600020918201910161245b9190612562565b5b505050565b60c06040519081016040528060008019168152602001600067ffffffffffffffff19168152602001600061ffff168152602001600061ffff168152602001600061ffff168152602001600081525090565b60c060405190810160405280600080191681526020016124d0612587565b815260200160008152602001600081525090565b604080519081016040528060008019168152602001600061ffff1681525090565b6060604051908101604052806000801916815260200160008019168152602001600060ff1681525090565b81548183558181111561255d5760030281600302836000526020600020918201910161255c91906125b2565b5b505050565b61258491905b80821115612580576000816000905550600101612568565b5090565b90565b6060604051908101604052806000801916815260200160008019168152602001600060ff1681525090565b6125f291905b808211156125ee5760008082016000905560018201600090556002820160006101000a81549060ff0219169055506003016125b8565b5090565b905600a165627a7a7230582002251e7a8af1aad202fb0c6ec11bba33fa41ee4690aa0557630bbcefc62410dd0029";

    public static final String FUNC_CHECKROLE = "checkRole";

    public static final String FUNC_ROLE_WHITELISTED = "ROLE_WHITELISTED";

    public static final String FUNC_HASROLE = "hasRole";

    public static final String FUNC_ADDCODE = "addCode";

    public static final String FUNC_REMOVEADDRESSESFROMWHITELIST = "removeAddressesFromWhitelist";

    public static final String FUNC_REMOVEADDRESSFROMWHITELIST = "removeAddressFromWhitelist";

    public static final String FUNC_GETSTATUS = "getStatus";

    public static final String FUNC_RENOUNCEOWNERSHIP = "renounceOwnership";

    public static final String FUNC_ADDADDRESSTOWHITELIST = "addAddressToWhitelist";

    public static final String FUNC_ADDNODE = "addNode";

    public static final String FUNC_OWNER = "owner";

    public static final String FUNC_WHITELIST = "whitelist";

    public static final String FUNC_GETNODECLUSTERS = "getNodeClusters";

    public static final String FUNC_GETCLUSTER = "getCluster";

    public static final String FUNC_ADDADDRESSESTOWHITELIST = "addAddressesToWhitelist";

    public static final String FUNC_TRANSFEROWNERSHIP = "transferOwnership";

    public static final Event CLUSTERFORMED_EVENT = new Event("ClusterFormed", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Bytes32>() {}, new TypeReference<Bytes32>() {}, new TypeReference<Uint256>() {}, new TypeReference<DynamicArray<Bytes32>>() {}, new TypeReference<DynamicArray<Bytes24>>() {}, new TypeReference<DynamicArray<Uint16>>() {}));
    ;

    public static final Event CODEENQUEUED_EVENT = new Event("CodeEnqueued", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Bytes32>() {}));
    ;

    public static final Event NEWNODE_EVENT = new Event("NewNode", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Bytes32>() {}));
    ;

    public static final Event ROLEADDED_EVENT = new Event("RoleAdded", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Address>(true) {}, new TypeReference<Utf8String>() {}));
    ;

    public static final Event ROLEREMOVED_EVENT = new Event("RoleRemoved", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Address>(true) {}, new TypeReference<Utf8String>() {}));
    ;

    public static final Event OWNERSHIPRENOUNCED_EVENT = new Event("OwnershipRenounced", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Address>(true) {}));
    ;

    public static final Event OWNERSHIPTRANSFERRED_EVENT = new Event("OwnershipTransferred", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Address>(true) {}, new TypeReference<Address>(true) {}));
    ;

    @Deprecated
    protected Deployer(String contractAddress, Web3j web3j, Credentials credentials, BigInteger gasPrice, BigInteger gasLimit) {
        super(BINARY, contractAddress, web3j, credentials, gasPrice, gasLimit);
    }

    protected Deployer(String contractAddress, Web3j web3j, Credentials credentials, ContractGasProvider contractGasProvider) {
        super(BINARY, contractAddress, web3j, credentials, contractGasProvider);
    }

    @Deprecated
    protected Deployer(String contractAddress, Web3j web3j, TransactionManager transactionManager, BigInteger gasPrice, BigInteger gasLimit) {
        super(BINARY, contractAddress, web3j, transactionManager, gasPrice, gasLimit);
    }

    protected Deployer(String contractAddress, Web3j web3j, TransactionManager transactionManager, ContractGasProvider contractGasProvider) {
        super(BINARY, contractAddress, web3j, transactionManager, contractGasProvider);
    }

    public void checkRole(Address _operator, Utf8String _role) {
        throw new RuntimeException("cannot call constant function with void return type");
    }

    public RemoteCall<Utf8String> ROLE_WHITELISTED() {
        final Function function = new Function(FUNC_ROLE_WHITELISTED, 
                Arrays.<Type>asList(), 
                Arrays.<TypeReference<?>>asList(new TypeReference<Utf8String>() {}));
        return executeRemoteCallSingleValueReturn(function);
    }

    public RemoteCall<Bool> hasRole(Address _operator, Utf8String _role) {
        final Function function = new Function(FUNC_HASROLE, 
                Arrays.<Type>asList(_operator, _role), 
                Arrays.<TypeReference<?>>asList(new TypeReference<Bool>() {}));
        return executeRemoteCallSingleValueReturn(function);
    }

    public RemoteCall<TransactionReceipt> addCode(Bytes32 storageHash, Bytes32 storageReceipt, Uint8 clusterSize) {
        final Function function = new Function(
                FUNC_ADDCODE, 
                Arrays.<Type>asList(storageHash, storageReceipt, clusterSize), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public RemoteCall<TransactionReceipt> removeAddressesFromWhitelist(DynamicArray<Address> _operators) {
        final Function function = new Function(
                FUNC_REMOVEADDRESSESFROMWHITELIST, 
                Arrays.<Type>asList(_operators), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public RemoteCall<TransactionReceipt> removeAddressFromWhitelist(Address _operator) {
        final Function function = new Function(
                FUNC_REMOVEADDRESSFROMWHITELIST, 
                Arrays.<Type>asList(_operator), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public RemoteCall<Tuple3<Uint8, Uint256, DynamicArray<Uint256>>> getStatus() {
        final Function function = new Function(FUNC_GETSTATUS, 
                Arrays.<Type>asList(), 
                Arrays.<TypeReference<?>>asList(new TypeReference<Uint8>() {}, new TypeReference<Uint256>() {}, new TypeReference<DynamicArray<Uint256>>() {}));
        return new RemoteCall<Tuple3<Uint8, Uint256, DynamicArray<Uint256>>>(
                new Callable<Tuple3<Uint8, Uint256, DynamicArray<Uint256>>>() {
                    @Override
                    public Tuple3<Uint8, Uint256, DynamicArray<Uint256>> call() throws Exception {
                        List<Type> results = executeCallMultipleValueReturn(function);
                        return new Tuple3<Uint8, Uint256, DynamicArray<Uint256>>(
                                (Uint8) results.get(0), 
                                (Uint256) results.get(1), 
                                (DynamicArray<Uint256>) results.get(2));
                    }
                });
    }

    public RemoteCall<TransactionReceipt> renounceOwnership() {
        final Function function = new Function(
                FUNC_RENOUNCEOWNERSHIP, 
                Arrays.<Type>asList(), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public RemoteCall<TransactionReceipt> addAddressToWhitelist(Address _operator) {
        final Function function = new Function(
                FUNC_ADDADDRESSTOWHITELIST, 
                Arrays.<Type>asList(_operator), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public RemoteCall<TransactionReceipt> addNode(Bytes32 nodeID, Bytes24 nodeAddress, Uint16 startPort, Uint16 endPort) {
        final Function function = new Function(
                FUNC_ADDNODE, 
                Arrays.<Type>asList(nodeID, nodeAddress, startPort, endPort), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public RemoteCall<Address> owner() {
        final Function function = new Function(FUNC_OWNER, 
                Arrays.<Type>asList(), 
                Arrays.<TypeReference<?>>asList(new TypeReference<Address>() {}));
        return executeRemoteCallSingleValueReturn(function);
    }

    public RemoteCall<Bool> whitelist(Address _operator) {
        final Function function = new Function(FUNC_WHITELIST, 
                Arrays.<Type>asList(_operator), 
                Arrays.<TypeReference<?>>asList(new TypeReference<Bool>() {}));
        return executeRemoteCallSingleValueReturn(function);
    }

    public RemoteCall<DynamicArray<Bytes32>> getNodeClusters(Bytes32 nodeID) {
        final Function function = new Function(FUNC_GETNODECLUSTERS, 
                Arrays.<Type>asList(nodeID), 
                Arrays.<TypeReference<?>>asList(new TypeReference<DynamicArray<Bytes32>>() {}));
        return executeRemoteCallSingleValueReturn(function);
    }

    public RemoteCall<Tuple6<Bytes32, Bytes32, Uint256, DynamicArray<Bytes32>, DynamicArray<Bytes24>, DynamicArray<Uint16>>> getCluster(Bytes32 clusterID) {
        final Function function = new Function(FUNC_GETCLUSTER, 
                Arrays.<Type>asList(clusterID), 
                Arrays.<TypeReference<?>>asList(new TypeReference<Bytes32>() {}, new TypeReference<Bytes32>() {}, new TypeReference<Uint256>() {}, new TypeReference<DynamicArray<Bytes32>>() {}, new TypeReference<DynamicArray<Bytes24>>() {}, new TypeReference<DynamicArray<Uint16>>() {}));
        return new RemoteCall<Tuple6<Bytes32, Bytes32, Uint256, DynamicArray<Bytes32>, DynamicArray<Bytes24>, DynamicArray<Uint16>>>(
                new Callable<Tuple6<Bytes32, Bytes32, Uint256, DynamicArray<Bytes32>, DynamicArray<Bytes24>, DynamicArray<Uint16>>>() {
                    @Override
                    public Tuple6<Bytes32, Bytes32, Uint256, DynamicArray<Bytes32>, DynamicArray<Bytes24>, DynamicArray<Uint16>> call() throws Exception {
                        List<Type> results = executeCallMultipleValueReturn(function);
                        return new Tuple6<Bytes32, Bytes32, Uint256, DynamicArray<Bytes32>, DynamicArray<Bytes24>, DynamicArray<Uint16>>(
                                (Bytes32) results.get(0), 
                                (Bytes32) results.get(1), 
                                (Uint256) results.get(2), 
                                (DynamicArray<Bytes32>) results.get(3), 
                                (DynamicArray<Bytes24>) results.get(4), 
                                (DynamicArray<Uint16>) results.get(5));
                    }
                });
    }

    public RemoteCall<TransactionReceipt> addAddressesToWhitelist(DynamicArray<Address> _operators) {
        final Function function = new Function(
                FUNC_ADDADDRESSESTOWHITELIST, 
                Arrays.<Type>asList(_operators), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public RemoteCall<TransactionReceipt> transferOwnership(Address _newOwner) {
        final Function function = new Function(
                FUNC_TRANSFEROWNERSHIP, 
                Arrays.<Type>asList(_newOwner), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public List<ClusterFormedEventResponse> getClusterFormedEvents(TransactionReceipt transactionReceipt) {
        List<Contract.EventValuesWithLog> valueList = extractEventParametersWithLog(CLUSTERFORMED_EVENT, transactionReceipt);
        ArrayList<ClusterFormedEventResponse> responses = new ArrayList<ClusterFormedEventResponse>(valueList.size());
        for (Contract.EventValuesWithLog eventValues : valueList) {
            ClusterFormedEventResponse typedResponse = new ClusterFormedEventResponse();
            typedResponse.log = eventValues.getLog();
            typedResponse.clusterID = (Bytes32) eventValues.getNonIndexedValues().get(0);
            typedResponse.storageHash = (Bytes32) eventValues.getNonIndexedValues().get(1);
            typedResponse.genesisTime = (Uint256) eventValues.getNonIndexedValues().get(2);
            typedResponse.solverIDs = (DynamicArray<Bytes32>) eventValues.getNonIndexedValues().get(3);
            typedResponse.solverAddrs = (DynamicArray<Bytes24>) eventValues.getNonIndexedValues().get(4);
            typedResponse.solverPorts = (DynamicArray<Uint16>) eventValues.getNonIndexedValues().get(5);
            responses.add(typedResponse);
        }
        return responses;
    }

    public Observable<ClusterFormedEventResponse> clusterFormedEventObservable(EthFilter filter) {
        return web3j.ethLogObservable(filter).map(new Func1<Log, ClusterFormedEventResponse>() {
            @Override
            public ClusterFormedEventResponse call(Log log) {
                Contract.EventValuesWithLog eventValues = extractEventParametersWithLog(CLUSTERFORMED_EVENT, log);
                ClusterFormedEventResponse typedResponse = new ClusterFormedEventResponse();
                typedResponse.log = log;
                typedResponse.clusterID = (Bytes32) eventValues.getNonIndexedValues().get(0);
                typedResponse.storageHash = (Bytes32) eventValues.getNonIndexedValues().get(1);
                typedResponse.genesisTime = (Uint256) eventValues.getNonIndexedValues().get(2);
                typedResponse.solverIDs = (DynamicArray<Bytes32>) eventValues.getNonIndexedValues().get(3);
                typedResponse.solverAddrs = (DynamicArray<Bytes24>) eventValues.getNonIndexedValues().get(4);
                typedResponse.solverPorts = (DynamicArray<Uint16>) eventValues.getNonIndexedValues().get(5);
                return typedResponse;
            }
        });
    }

    public Observable<ClusterFormedEventResponse> clusterFormedEventObservable(DefaultBlockParameter startBlock, DefaultBlockParameter endBlock) {
        EthFilter filter = new EthFilter(startBlock, endBlock, getContractAddress());
        filter.addSingleTopic(EventEncoder.encode(CLUSTERFORMED_EVENT));
        return clusterFormedEventObservable(filter);
    }

    public List<CodeEnqueuedEventResponse> getCodeEnqueuedEvents(TransactionReceipt transactionReceipt) {
        List<Contract.EventValuesWithLog> valueList = extractEventParametersWithLog(CODEENQUEUED_EVENT, transactionReceipt);
        ArrayList<CodeEnqueuedEventResponse> responses = new ArrayList<CodeEnqueuedEventResponse>(valueList.size());
        for (Contract.EventValuesWithLog eventValues : valueList) {
            CodeEnqueuedEventResponse typedResponse = new CodeEnqueuedEventResponse();
            typedResponse.log = eventValues.getLog();
            typedResponse.storageHash = (Bytes32) eventValues.getNonIndexedValues().get(0);
            responses.add(typedResponse);
        }
        return responses;
    }

    public Observable<CodeEnqueuedEventResponse> codeEnqueuedEventObservable(EthFilter filter) {
        return web3j.ethLogObservable(filter).map(new Func1<Log, CodeEnqueuedEventResponse>() {
            @Override
            public CodeEnqueuedEventResponse call(Log log) {
                Contract.EventValuesWithLog eventValues = extractEventParametersWithLog(CODEENQUEUED_EVENT, log);
                CodeEnqueuedEventResponse typedResponse = new CodeEnqueuedEventResponse();
                typedResponse.log = log;
                typedResponse.storageHash = (Bytes32) eventValues.getNonIndexedValues().get(0);
                return typedResponse;
            }
        });
    }

    public Observable<CodeEnqueuedEventResponse> codeEnqueuedEventObservable(DefaultBlockParameter startBlock, DefaultBlockParameter endBlock) {
        EthFilter filter = new EthFilter(startBlock, endBlock, getContractAddress());
        filter.addSingleTopic(EventEncoder.encode(CODEENQUEUED_EVENT));
        return codeEnqueuedEventObservable(filter);
    }

    public List<NewNodeEventResponse> getNewNodeEvents(TransactionReceipt transactionReceipt) {
        List<Contract.EventValuesWithLog> valueList = extractEventParametersWithLog(NEWNODE_EVENT, transactionReceipt);
        ArrayList<NewNodeEventResponse> responses = new ArrayList<NewNodeEventResponse>(valueList.size());
        for (Contract.EventValuesWithLog eventValues : valueList) {
            NewNodeEventResponse typedResponse = new NewNodeEventResponse();
            typedResponse.log = eventValues.getLog();
            typedResponse.id = (Bytes32) eventValues.getNonIndexedValues().get(0);
            responses.add(typedResponse);
        }
        return responses;
    }

    public Observable<NewNodeEventResponse> newNodeEventObservable(EthFilter filter) {
        return web3j.ethLogObservable(filter).map(new Func1<Log, NewNodeEventResponse>() {
            @Override
            public NewNodeEventResponse call(Log log) {
                Contract.EventValuesWithLog eventValues = extractEventParametersWithLog(NEWNODE_EVENT, log);
                NewNodeEventResponse typedResponse = new NewNodeEventResponse();
                typedResponse.log = log;
                typedResponse.id = (Bytes32) eventValues.getNonIndexedValues().get(0);
                return typedResponse;
            }
        });
    }

    public Observable<NewNodeEventResponse> newNodeEventObservable(DefaultBlockParameter startBlock, DefaultBlockParameter endBlock) {
        EthFilter filter = new EthFilter(startBlock, endBlock, getContractAddress());
        filter.addSingleTopic(EventEncoder.encode(NEWNODE_EVENT));
        return newNodeEventObservable(filter);
    }

    public List<RoleAddedEventResponse> getRoleAddedEvents(TransactionReceipt transactionReceipt) {
        List<Contract.EventValuesWithLog> valueList = extractEventParametersWithLog(ROLEADDED_EVENT, transactionReceipt);
        ArrayList<RoleAddedEventResponse> responses = new ArrayList<RoleAddedEventResponse>(valueList.size());
        for (Contract.EventValuesWithLog eventValues : valueList) {
            RoleAddedEventResponse typedResponse = new RoleAddedEventResponse();
            typedResponse.log = eventValues.getLog();
            typedResponse.operator = (Address) eventValues.getIndexedValues().get(0);
            typedResponse.role = (Utf8String) eventValues.getNonIndexedValues().get(0);
            responses.add(typedResponse);
        }
        return responses;
    }

    public Observable<RoleAddedEventResponse> roleAddedEventObservable(EthFilter filter) {
        return web3j.ethLogObservable(filter).map(new Func1<Log, RoleAddedEventResponse>() {
            @Override
            public RoleAddedEventResponse call(Log log) {
                Contract.EventValuesWithLog eventValues = extractEventParametersWithLog(ROLEADDED_EVENT, log);
                RoleAddedEventResponse typedResponse = new RoleAddedEventResponse();
                typedResponse.log = log;
                typedResponse.operator = (Address) eventValues.getIndexedValues().get(0);
                typedResponse.role = (Utf8String) eventValues.getNonIndexedValues().get(0);
                return typedResponse;
            }
        });
    }

    public Observable<RoleAddedEventResponse> roleAddedEventObservable(DefaultBlockParameter startBlock, DefaultBlockParameter endBlock) {
        EthFilter filter = new EthFilter(startBlock, endBlock, getContractAddress());
        filter.addSingleTopic(EventEncoder.encode(ROLEADDED_EVENT));
        return roleAddedEventObservable(filter);
    }

    public List<RoleRemovedEventResponse> getRoleRemovedEvents(TransactionReceipt transactionReceipt) {
        List<Contract.EventValuesWithLog> valueList = extractEventParametersWithLog(ROLEREMOVED_EVENT, transactionReceipt);
        ArrayList<RoleRemovedEventResponse> responses = new ArrayList<RoleRemovedEventResponse>(valueList.size());
        for (Contract.EventValuesWithLog eventValues : valueList) {
            RoleRemovedEventResponse typedResponse = new RoleRemovedEventResponse();
            typedResponse.log = eventValues.getLog();
            typedResponse.operator = (Address) eventValues.getIndexedValues().get(0);
            typedResponse.role = (Utf8String) eventValues.getNonIndexedValues().get(0);
            responses.add(typedResponse);
        }
        return responses;
    }

    public Observable<RoleRemovedEventResponse> roleRemovedEventObservable(EthFilter filter) {
        return web3j.ethLogObservable(filter).map(new Func1<Log, RoleRemovedEventResponse>() {
            @Override
            public RoleRemovedEventResponse call(Log log) {
                Contract.EventValuesWithLog eventValues = extractEventParametersWithLog(ROLEREMOVED_EVENT, log);
                RoleRemovedEventResponse typedResponse = new RoleRemovedEventResponse();
                typedResponse.log = log;
                typedResponse.operator = (Address) eventValues.getIndexedValues().get(0);
                typedResponse.role = (Utf8String) eventValues.getNonIndexedValues().get(0);
                return typedResponse;
            }
        });
    }

    public Observable<RoleRemovedEventResponse> roleRemovedEventObservable(DefaultBlockParameter startBlock, DefaultBlockParameter endBlock) {
        EthFilter filter = new EthFilter(startBlock, endBlock, getContractAddress());
        filter.addSingleTopic(EventEncoder.encode(ROLEREMOVED_EVENT));
        return roleRemovedEventObservable(filter);
    }

    public List<OwnershipRenouncedEventResponse> getOwnershipRenouncedEvents(TransactionReceipt transactionReceipt) {
        List<Contract.EventValuesWithLog> valueList = extractEventParametersWithLog(OWNERSHIPRENOUNCED_EVENT, transactionReceipt);
        ArrayList<OwnershipRenouncedEventResponse> responses = new ArrayList<OwnershipRenouncedEventResponse>(valueList.size());
        for (Contract.EventValuesWithLog eventValues : valueList) {
            OwnershipRenouncedEventResponse typedResponse = new OwnershipRenouncedEventResponse();
            typedResponse.log = eventValues.getLog();
            typedResponse.previousOwner = (Address) eventValues.getIndexedValues().get(0);
            responses.add(typedResponse);
        }
        return responses;
    }

    public Observable<OwnershipRenouncedEventResponse> ownershipRenouncedEventObservable(EthFilter filter) {
        return web3j.ethLogObservable(filter).map(new Func1<Log, OwnershipRenouncedEventResponse>() {
            @Override
            public OwnershipRenouncedEventResponse call(Log log) {
                Contract.EventValuesWithLog eventValues = extractEventParametersWithLog(OWNERSHIPRENOUNCED_EVENT, log);
                OwnershipRenouncedEventResponse typedResponse = new OwnershipRenouncedEventResponse();
                typedResponse.log = log;
                typedResponse.previousOwner = (Address) eventValues.getIndexedValues().get(0);
                return typedResponse;
            }
        });
    }

    public Observable<OwnershipRenouncedEventResponse> ownershipRenouncedEventObservable(DefaultBlockParameter startBlock, DefaultBlockParameter endBlock) {
        EthFilter filter = new EthFilter(startBlock, endBlock, getContractAddress());
        filter.addSingleTopic(EventEncoder.encode(OWNERSHIPRENOUNCED_EVENT));
        return ownershipRenouncedEventObservable(filter);
    }

    public List<OwnershipTransferredEventResponse> getOwnershipTransferredEvents(TransactionReceipt transactionReceipt) {
        List<Contract.EventValuesWithLog> valueList = extractEventParametersWithLog(OWNERSHIPTRANSFERRED_EVENT, transactionReceipt);
        ArrayList<OwnershipTransferredEventResponse> responses = new ArrayList<OwnershipTransferredEventResponse>(valueList.size());
        for (Contract.EventValuesWithLog eventValues : valueList) {
            OwnershipTransferredEventResponse typedResponse = new OwnershipTransferredEventResponse();
            typedResponse.log = eventValues.getLog();
            typedResponse.previousOwner = (Address) eventValues.getIndexedValues().get(0);
            typedResponse.newOwner = (Address) eventValues.getIndexedValues().get(1);
            responses.add(typedResponse);
        }
        return responses;
    }

    public Observable<OwnershipTransferredEventResponse> ownershipTransferredEventObservable(EthFilter filter) {
        return web3j.ethLogObservable(filter).map(new Func1<Log, OwnershipTransferredEventResponse>() {
            @Override
            public OwnershipTransferredEventResponse call(Log log) {
                Contract.EventValuesWithLog eventValues = extractEventParametersWithLog(OWNERSHIPTRANSFERRED_EVENT, log);
                OwnershipTransferredEventResponse typedResponse = new OwnershipTransferredEventResponse();
                typedResponse.log = log;
                typedResponse.previousOwner = (Address) eventValues.getIndexedValues().get(0);
                typedResponse.newOwner = (Address) eventValues.getIndexedValues().get(1);
                return typedResponse;
            }
        });
    }

    public Observable<OwnershipTransferredEventResponse> ownershipTransferredEventObservable(DefaultBlockParameter startBlock, DefaultBlockParameter endBlock) {
        EthFilter filter = new EthFilter(startBlock, endBlock, getContractAddress());
        filter.addSingleTopic(EventEncoder.encode(OWNERSHIPTRANSFERRED_EVENT));
        return ownershipTransferredEventObservable(filter);
    }

    public static RemoteCall<Deployer> deploy(Web3j web3j, Credentials credentials, ContractGasProvider contractGasProvider) {
        return deployRemoteCall(Deployer.class, web3j, credentials, contractGasProvider, BINARY, "");
    }

    @Deprecated
    public static RemoteCall<Deployer> deploy(Web3j web3j, Credentials credentials, BigInteger gasPrice, BigInteger gasLimit) {
        return deployRemoteCall(Deployer.class, web3j, credentials, gasPrice, gasLimit, BINARY, "");
    }

    public static RemoteCall<Deployer> deploy(Web3j web3j, TransactionManager transactionManager, ContractGasProvider contractGasProvider) {
        return deployRemoteCall(Deployer.class, web3j, transactionManager, contractGasProvider, BINARY, "");
    }

    @Deprecated
    public static RemoteCall<Deployer> deploy(Web3j web3j, TransactionManager transactionManager, BigInteger gasPrice, BigInteger gasLimit) {
        return deployRemoteCall(Deployer.class, web3j, transactionManager, gasPrice, gasLimit, BINARY, "");
    }

    @Deprecated
    public static Deployer load(String contractAddress, Web3j web3j, Credentials credentials, BigInteger gasPrice, BigInteger gasLimit) {
        return new Deployer(contractAddress, web3j, credentials, gasPrice, gasLimit);
    }

    @Deprecated
    public static Deployer load(String contractAddress, Web3j web3j, TransactionManager transactionManager, BigInteger gasPrice, BigInteger gasLimit) {
        return new Deployer(contractAddress, web3j, transactionManager, gasPrice, gasLimit);
    }

    public static Deployer load(String contractAddress, Web3j web3j, Credentials credentials, ContractGasProvider contractGasProvider) {
        return new Deployer(contractAddress, web3j, credentials, contractGasProvider);
    }

    public static Deployer load(String contractAddress, Web3j web3j, TransactionManager transactionManager, ContractGasProvider contractGasProvider) {
        return new Deployer(contractAddress, web3j, transactionManager, contractGasProvider);
    }

    public static class ClusterFormedEventResponse {
        public Log log;

        public Bytes32 clusterID;

        public Bytes32 storageHash;

        public Uint256 genesisTime;

        public DynamicArray<Bytes32> solverIDs;

        public DynamicArray<Bytes24> solverAddrs;

        public DynamicArray<Uint16> solverPorts;
    }

    public static class CodeEnqueuedEventResponse {
        public Log log;

        public Bytes32 storageHash;
    }

    public static class NewNodeEventResponse {
        public Log log;

        public Bytes32 id;
    }

    public static class RoleAddedEventResponse {
        public Log log;

        public Address operator;

        public Utf8String role;
    }

    public static class RoleRemovedEventResponse {
        public Log log;

        public Address operator;

        public Utf8String role;
    }

    public static class OwnershipRenouncedEventResponse {
        public Log log;

        public Address previousOwner;
    }

    public static class OwnershipTransferredEventResponse {
        public Log log;

        public Address previousOwner;

        public Address newOwner;
    }
}
