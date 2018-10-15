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
import org.web3j.abi.datatypes.generated.Bytes32;
import org.web3j.abi.datatypes.generated.Uint8;
import org.web3j.crypto.Credentials;
import org.web3j.protocol.Web3j;
import org.web3j.protocol.core.DefaultBlockParameter;
import org.web3j.protocol.core.RemoteCall;
import org.web3j.protocol.core.methods.request.EthFilter;
import org.web3j.protocol.core.methods.response.Log;
import org.web3j.protocol.core.methods.response.TransactionReceipt;
import org.web3j.tuples.generated.Tuple2;
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
    private static final String BINARY = "60806040526001600555336000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055506118e9806100586000396000f3006080604052600436106100d0576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff1680630988ca8c146100d557806318b919e91461015e578063217fe6c6146101ee5780632238ba2f1461028f57806324953eaa146102db578063286dd3f5146103415780632bb62ae014610384578063715018a6146103c35780637b9417c8146103da5780638da5cb5b1461041d5780639b19251a14610474578063e2ec6ec3146104cf578063e8fe6b6614610535578063f2fde38b14610591575b600080fd5b3480156100e157600080fd5b5061015c600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803590602001908201803590602001908080601f01602080910402602001604051908101604052809392919081815260200183838082843782019150505050505091929192905050506105d4565b005b34801561016a57600080fd5b50610173610655565b6040518080602001828103825283818151815260200191508051906020019080838360005b838110156101b3578082015181840152602081019050610198565b50505050905090810190601f1680156101e05780820380516001836020036101000a031916815260200191505b509250505060405180910390f35b3480156101fa57600080fd5b50610275600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190803590602001908201803590602001908080601f016020809104026020016040519081016040528093929190818152602001838380828437820191505050505050919291929050505061068e565b604051808215151515815260200191505060405180910390f35b34801561029b57600080fd5b506102d960048036038101908080356000191690602001909291908035600019169060200190929190803560ff169060200190929190505050610715565b005b3480156102e757600080fd5b5061033f60048036038101908080359060200190820180359060200190808060200260200160405190810160405280939291908181526020018383602002808284378201915050505050509192919290505050610841565b005b34801561034d57600080fd5b50610382600480360381019080803573ffffffffffffffffffffffffffffffffffffffff1690602001909291905050506108dd565b005b34801561039057600080fd5b506103c16004803603810190808035600019169060200190929190803560001916906020019092919050505061097a565b005b3480156103cf57600080fd5b506103d8610b57565b005b3480156103e657600080fd5b5061041b600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610c59565b005b34801561042957600080fd5b50610432610cf6565b604051808273ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200191505060405180910390f35b34801561048057600080fd5b506104b5600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610d1b565b604051808215151515815260200191505060405180910390f35b3480156104db57600080fd5b5061053360048036038101908080359060200190820180359060200190808060200260200160405190810160405280939291908181526020018383602002808284378201915050505050509192919290505050610d63565b005b34801561054157600080fd5b506105646004803603810190808035600019169060200190929190505050610dff565b60405180836000191660001916815260200182600019166000191681526020019250505060405180910390f35b34801561059d57600080fd5b506105d2600480360381019080803573ffffffffffffffffffffffffffffffffffffffff169060200190929190505050610f39565b005b610651826001836040518082805190602001908083835b60208310151561061057805182526020820191506020810190506020830392506105eb565b6001836020036101000a0380198251168184511680821785525050505050509050019150509081526020016040518091039020610fa090919063ffffffff16565b5050565b6040805190810160405280600981526020017f77686974656c697374000000000000000000000000000000000000000000000081525081565b600061070d836001846040518082805190602001908083835b6020831015156106cc57805182526020820191506020810190506020830392506106a7565b6001836020036101000a0380198251168184511680821785525050505050509050019150509081526020016040518091039020610fb990919063ffffffff16565b905092915050565b33610755816040805190810160405280600981526020017f77686974656c69737400000000000000000000000000000000000000000000008152506105d4565b600660606040519081016040528086600019168152602001856000191681526020018460ff1681525090806001815401808255809150509060018203906000526020600020906003020160009091929091909150600082015181600001906000191690556020820151816001019060001916905560408201518160020160006101000a81548160ff021916908360ff1602179055505050506107f5611012565b151561083b577fd18fba5b22517a48b063e62f8b6acbfc4dbfba1583e929178d3fc862218544dd8460405180826000191660001916815260200191505060405180910390a15b50505050565b60008060009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff1614151561089e57600080fd5b600090505b81518110156108d9576108cc82828151811015156108bd57fe5b906020019060200201516108dd565b80806001019150506108a3565b5050565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff1614151561093857600080fd5b610977816040805190810160405280600981526020017f77686974656c69737400000000000000000000000000000000000000000000008152506113d4565b50565b336109ba816040805190810160405280600981526020017f77686974656c69737400000000000000000000000000000000000000000000008152506105d4565b60006001026003600085600019166000191681526020019081526020016000206000015460001916141515610a7d576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260218152602001807f5468697320736f6c76657220697320616c72656164792072656769737465726581526020017f640000000000000000000000000000000000000000000000000000000000000081525060400191505060405180910390fd5b6040805190810160405280846000191681526020018360001916815250600360008560001916600019168152602001908152602001600020600082015181600001906000191690556020820151816001019060001916905590505060028390806001815401808255809150509060018203906000526020600020016000909192909190915090600019169055507fc6aa0f5a4d613646f930c3c4d9c6a4f213549c605897bf0ae5c92f914559a0118360405180826000191660001916815260200191505060405180910390a1610b51611012565b50505050565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16141515610bb257600080fd5b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff167ff8df31144d9c2f0f6b59d69b8b98abd5459d07f2742c4df920b25aae33c6482060405160405180910390a260008060006101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff160217905550565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16141515610cb457600080fd5b610cf3816040805190810160405280600981526020017f77686974656c6973740000000000000000000000000000000000000000000000815250611508565b50565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b6000610d5c826040805190810160405280600981526020017f77686974656c697374000000000000000000000000000000000000000000000081525061068e565b9050919050565b60008060009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16141515610dc057600080fd5b600090505b8151811015610dfb57610dee8282815181101515610ddf57fe5b90602001906020020151610c59565b8080600101915050610dc5565b5050565b600080610e0a6117f2565b60046000856000191660001916815260200190815260200160002060408051908101604052908160008201546000191660001916815260200160018201606060405190810160405290816000820154600019166000191681526020016001820154600019166000191681526020016002820160009054906101000a900460ff1660ff1660ff16815250508152505090506000600102816000015160001916111515610f1d576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004018080602001828103825260188152602001807f7468657265206973206e6f207375636820636c7573746572000000000000000081525060200191505060405180910390fd5b8060200151600001518160200151602001519250925050915091565b6000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16141515610f9457600080fd5b610f9d8161163c565b50565b610faa8282610fb9565b1515610fb557600080fd5b5050565b60008260000160008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060009054906101000a900460ff16905092915050565b60008061101d611816565b60608060008060008096505b6006805490508710156110845760068781548110151561104557fe5b906000526020600020906003020160020160009054906101000a900460ff1660ff1660028054905010151561107957611084565b866001019650611029565b6006805490508710151561109b57600097506113ca565b6006878154811015156110aa57fe5b9060005260206000209060030201606060405190810160405290816000820154600019166000191681526020016001820154600019166000191681526020016002820160009054906101000a900460ff1660ff1660ff16815250509550856040015160ff1660405190808252806020026020018201604052801561113d5781602001602082028038833980820191505090505b509450856040015160ff166040519080825280602002602001820160405280156111765781602001602082028038833980820191505090505b509350600092505b856040015160ff1683101561122c57600260018460028054905003038154811015156111a657fe5b906000526020600020015491508185848151811015156111c257fe5b90602001906020020190600019169081600019168152505060036000836000191660001916815260200190815260200160002060010154848481518110151561120757fe5b906020019060200201906000191690816000191681525050828060010193505061117e565b856040015160ff1660028181805490500391508161124a9190611841565b50600560008154809291906001019190505560010290506040805190810160405280826000191681526020018781525060046000836000191660001916815260200190815260200160002060008201518160000190600019169055602082015181600101600082015181600001906000191690556020820151816001019060001916905560408201518160020160006101000a81548160ff021916908360ff16021790555050509050507f4c99ecb5fd2810b2ecde0e23958d1c8a5e626342d58fecc3fa9ce945887567048186866040518084600019166000191681526020018060200180602001838103835285818151815260200191508051906020019060200280838360005b8381101561136d578082015181840152602081019050611352565b50505050905001838103825284818151815260200191508051906020019060200280838360005b838110156113af578082015181840152602081019050611394565b505050509050019550505050505060405180910390a1600197505b5050505050505090565b611451826001836040518082805190602001908083835b60208310151561141057805182526020820191506020810190506020830392506113eb565b6001836020036101000a038019825116818451168082178552505050505050905001915050908152602001604051809103902061173690919063ffffffff16565b8173ffffffffffffffffffffffffffffffffffffffff167fd211483f91fc6eff862467f8de606587a30c8fc9981056f051b897a418df803a826040518080602001828103825283818151815260200191508051906020019080838360005b838110156114ca5780820151818401526020810190506114af565b50505050905090810190601f1680156114f75780820380516001836020036101000a031916815260200191505b509250505060405180910390a25050565b611585826001836040518082805190602001908083835b602083101515611544578051825260208201915060208101905060208303925061151f565b6001836020036101000a038019825116818451168082178552505050505050905001915050908152602001604051809103902061179490919063ffffffff16565b8173ffffffffffffffffffffffffffffffffffffffff167fbfec83d64eaa953f2708271a023ab9ee82057f8f3578d548c1a4ba0b5b700489826040518080602001828103825283818151815260200191508051906020019080838360005b838110156115fe5780820151818401526020810190506115e3565b50505050905090810190601f16801561162b5780820380516001836020036101000a031916815260200191505b509250505060405180910390a25050565b600073ffffffffffffffffffffffffffffffffffffffff168173ffffffffffffffffffffffffffffffffffffffff161415151561167857600080fd5b8073ffffffffffffffffffffffffffffffffffffffff166000809054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff167f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e060405160405180910390a3806000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555050565b60008260000160008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060006101000a81548160ff0219169083151502179055505050565b60018260000160008373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff16815260200190815260200160002060006101000a81548160ff0219169083151502179055505050565b6080604051908101604052806000801916815260200161181061186d565b81525090565b6060604051908101604052806000801916815260200160008019168152602001600060ff1681525090565b815481835581811115611868578183600052602060002091820191016118679190611898565b5b505050565b6060604051908101604052806000801916815260200160008019168152602001600060ff1681525090565b6118ba91905b808211156118b657600081600090555060010161189e565b5090565b905600a165627a7a72305820f7f2683c9cd45f316c4660ab659b3e966ec51b6a0c65618a69b49cebc498c53d0029";

    public static final String FUNC_CHECKROLE = "checkRole";

    public static final String FUNC_ROLE_WHITELISTED = "ROLE_WHITELISTED";

    public static final String FUNC_HASROLE = "hasRole";

    public static final String FUNC_ADDCODE = "addCode";

    public static final String FUNC_REMOVEADDRESSESFROMWHITELIST = "removeAddressesFromWhitelist";

    public static final String FUNC_REMOVEADDRESSFROMWHITELIST = "removeAddressFromWhitelist";

    public static final String FUNC_ADDSOLVER = "addSolver";

    public static final String FUNC_RENOUNCEOWNERSHIP = "renounceOwnership";

    public static final String FUNC_ADDADDRESSTOWHITELIST = "addAddressToWhitelist";

    public static final String FUNC_OWNER = "owner";

    public static final String FUNC_WHITELIST = "whitelist";

    public static final String FUNC_ADDADDRESSESTOWHITELIST = "addAddressesToWhitelist";

    public static final String FUNC_GETCODE = "getCode";

    public static final String FUNC_TRANSFEROWNERSHIP = "transferOwnership";

    public static final Event CLUSTERFORMED_EVENT = new Event("ClusterFormed", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Bytes32>() {}, new TypeReference<DynamicArray<Bytes32>>() {}, new TypeReference<DynamicArray<Bytes32>>() {}));
    ;

    public static final Event CODEENQUEUED_EVENT = new Event("CodeEnqueued", 
            Arrays.<TypeReference<?>>asList(new TypeReference<Bytes32>() {}));
    ;

    public static final Event NEWSOLVER_EVENT = new Event("NewSolver", 
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

    public RemoteCall<TransactionReceipt> addSolver(Bytes32 solverID, Bytes32 solverAddress) {
        final Function function = new Function(
                FUNC_ADDSOLVER, 
                Arrays.<Type>asList(solverID, solverAddress), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
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

    public RemoteCall<TransactionReceipt> addAddressesToWhitelist(DynamicArray<Address> _operators) {
        final Function function = new Function(
                FUNC_ADDADDRESSESTOWHITELIST, 
                Arrays.<Type>asList(_operators), 
                Collections.<TypeReference<?>>emptyList());
        return executeRemoteCallTransaction(function);
    }

    public RemoteCall<Tuple2<Bytes32, Bytes32>> getCode(Bytes32 clusterID) {
        final Function function = new Function(FUNC_GETCODE, 
                Arrays.<Type>asList(clusterID), 
                Arrays.<TypeReference<?>>asList(new TypeReference<Bytes32>() {}, new TypeReference<Bytes32>() {}));
        return new RemoteCall<Tuple2<Bytes32, Bytes32>>(
                new Callable<Tuple2<Bytes32, Bytes32>>() {
                    @Override
                    public Tuple2<Bytes32, Bytes32> call() throws Exception {
                        List<Type> results = executeCallMultipleValueReturn(function);
                        return new Tuple2<Bytes32, Bytes32>(
                                (Bytes32) results.get(0), 
                                (Bytes32) results.get(1));
                    }
                });
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
            typedResponse.solverIDs = (DynamicArray<Bytes32>) eventValues.getNonIndexedValues().get(1);
            typedResponse.solverAddresses = (DynamicArray<Bytes32>) eventValues.getNonIndexedValues().get(2);
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
                typedResponse.solverIDs = (DynamicArray<Bytes32>) eventValues.getNonIndexedValues().get(1);
                typedResponse.solverAddresses = (DynamicArray<Bytes32>) eventValues.getNonIndexedValues().get(2);
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

    public List<NewSolverEventResponse> getNewSolverEvents(TransactionReceipt transactionReceipt) {
        List<Contract.EventValuesWithLog> valueList = extractEventParametersWithLog(NEWSOLVER_EVENT, transactionReceipt);
        ArrayList<NewSolverEventResponse> responses = new ArrayList<NewSolverEventResponse>(valueList.size());
        for (Contract.EventValuesWithLog eventValues : valueList) {
            NewSolverEventResponse typedResponse = new NewSolverEventResponse();
            typedResponse.log = eventValues.getLog();
            typedResponse.id = (Bytes32) eventValues.getNonIndexedValues().get(0);
            responses.add(typedResponse);
        }
        return responses;
    }

    public Observable<NewSolverEventResponse> newSolverEventObservable(EthFilter filter) {
        return web3j.ethLogObservable(filter).map(new Func1<Log, NewSolverEventResponse>() {
            @Override
            public NewSolverEventResponse call(Log log) {
                Contract.EventValuesWithLog eventValues = extractEventParametersWithLog(NEWSOLVER_EVENT, log);
                NewSolverEventResponse typedResponse = new NewSolverEventResponse();
                typedResponse.log = log;
                typedResponse.id = (Bytes32) eventValues.getNonIndexedValues().get(0);
                return typedResponse;
            }
        });
    }

    public Observable<NewSolverEventResponse> newSolverEventObservable(DefaultBlockParameter startBlock, DefaultBlockParameter endBlock) {
        EthFilter filter = new EthFilter(startBlock, endBlock, getContractAddress());
        filter.addSingleTopic(EventEncoder.encode(NEWSOLVER_EVENT));
        return newSolverEventObservable(filter);
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

        public DynamicArray<Bytes32> solverIDs;

        public DynamicArray<Bytes32> solverAddresses;
    }

    public static class CodeEnqueuedEventResponse {
        public Log log;

        public Bytes32 storageHash;
    }

    public static class NewSolverEventResponse {
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
